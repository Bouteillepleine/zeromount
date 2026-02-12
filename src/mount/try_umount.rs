use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Once;

use anyhow::{bail, Result};
use tracing::{debug, info, warn};

const KSU_MAGIC1: u32 = 0xDEADBEEF;
const KSU_MAGIC2: u32 = 0xCAFEBABE;

// _IOC(_IOC_WRITE, 'K', 18, 0)
const KSU_IOCTL_ADD_TRY_UMOUNT: u32 = 0x4000_4B12;

// KSU infrastructure mounts that detection tools flag but modules can't prevent.
// Parent paths included because KSU may create directory-level mounts too.
// try_umount silently skips non-mount-point paths, so extras are zero-cost.
const KSU_INFRA_PATHS: &[&str] = &[
    "/apex/com.android.art/javalib/core-libart.jar",
    "/apex/com.android.art/javalib",
    "/apex/com.android.art",
];

#[repr(C)]
struct KsuTryUmountCmd {
    arg: u64,
    flags: u32,
    mode: u8,
}

static DRIVER_FD: AtomicI32 = AtomicI32::new(-1);
static INIT: Once = Once::new();

fn acquire_driver_fd() -> i32 {
    INIT.call_once(|| {
        let mut fd: i32 = -1;
        let ret = unsafe {
            libc::syscall(
                libc::SYS_reboot,
                KSU_MAGIC1 as libc::c_long,
                KSU_MAGIC2 as libc::c_long,
                0 as libc::c_long,
                &mut fd as *mut i32 as libc::c_long,
            )
        };
        // KSU writes fd via pointer regardless of syscall return value.
        // Bare KSU kernels return ret=-1 from the supercall but fd IS valid.
        // SUSFS-patched kernels return ret=0. Check fd only.
        if fd >= 0 {
            debug!(fd, ret, "KSU driver fd acquired");
            DRIVER_FD.store(fd, Ordering::Release);
        } else {
            warn!(ret, fd, "KSU driver fd acquisition failed");
        }
    });
    DRIVER_FD.load(Ordering::Acquire)
}

fn send_unmountable(path: &str) -> Result<()> {
    let fd = acquire_driver_fd();
    if fd < 0 {
        bail!("KSU driver not available");
    }

    let c_path = std::ffi::CString::new(path)?;
    let cmd = KsuTryUmountCmd {
        arg: c_path.as_ptr() as u64,
        flags: 0x2,
        mode: 1, // add_to_list
    };

    let ret = unsafe {
        libc::ioctl(fd, KSU_IOCTL_ADD_TRY_UMOUNT as libc::Ioctl, &cmd as *const KsuTryUmountCmd)
    };

    if ret < 0 {
        let err = std::io::Error::last_os_error();
        bail!("try_umount ioctl failed for {path}: {err}");
    }

    debug!(path, "registered with try_umount");
    Ok(())
}

pub struct TryUmountStats {
    pub registered: u32,
    pub failed: u32,
}

/// Register mount paths with KSU's try_umount for per-app unmounting.
/// KSU reverses these mounts in the mount namespace of deny-list apps.
pub fn register_unmountable(mount_paths: &[String], root_manager_name: &str) -> TryUmountStats {
    if root_manager_name != "KernelSU" {
        debug!("try_umount skipped: root manager is {root_manager_name}, not KernelSU");
        return TryUmountStats { registered: 0, failed: 0 };
    }

    let mut registered = 0u32;
    let mut failed = 0u32;

    for path in mount_paths {
        match send_unmountable(path) {
            Ok(()) => registered += 1,
            Err(e) => {
                warn!(path = %path, error = %e, "try_umount registration failed");
                failed += 1;
            }
        }
    }

    if registered > 0 || failed > 0 {
        info!(registered, failed, "try_umount registration complete");
    }

    TryUmountStats { registered, failed }
}

// Scan init's mountinfo for KSU-created bind mounts at /apex paths.
// KSU injects ART modifications via bind mounts that aren't in our static list.
fn discover_ksu_infra_mounts() -> Vec<String> {
    let mountinfo = match std::fs::read_to_string("/proc/1/mountinfo") {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let mut paths = Vec::new();
    for line in mountinfo.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 10 {
            continue;
        }
        let root_field = fields[3];
        let mount_point = fields[4];

        let dash_pos = match fields.iter().position(|&f| f == "-") {
            Some(p) => p,
            None => continue,
        };
        let fs_type = fields.get(dash_pos + 1).copied().unwrap_or("");
        let source = fields.get(dash_pos + 2).copied().unwrap_or("");

        if !mount_point.starts_with("/apex/") {
            continue;
        }

        // KSU tmpfs mounts at /apex paths
        if source == "KSU" {
            paths.push(mount_point.to_string());
            continue;
        }

        // Bind mount overlay: non-"/" root on a non-tmpfs, non-overlay mount
        if root_field != "/" && fs_type != "tmpfs" && fs_type != "overlay" {
            paths.push(mount_point.to_string());
        }
    }

    if !paths.is_empty() {
        debug!(count = paths.len(), paths = ?paths, "discovered KSU infra mounts");
    }
    paths
}

// KSU bind-mounts core-libart.jar at zygote init — AFTER our post-fs-data
// binary runs. Register unconditionally so try_umount covers late mounts.
pub fn register_ksu_infra_mounts(root_manager_name: &str) -> TryUmountStats {
    if root_manager_name != "KernelSU" {
        return TryUmountStats { registered: 0, failed: 0 };
    }

    let mut registered = 0u32;
    let mut failed = 0u32;

    for &path in KSU_INFRA_PATHS {
        match send_unmountable(path) {
            Ok(()) => {
                debug!(path, "KSU infra path registered with try_umount");
                registered += 1;
            }
            Err(e) => {
                debug!(path, error = %e, "KSU infra try_umount failed");
                failed += 1;
            }
        }
    }

    for path in discover_ksu_infra_mounts() {
        if KSU_INFRA_PATHS.contains(&path.as_str()) {
            continue;
        }
        match send_unmountable(&path) {
            Ok(()) => {
                debug!(path = %path, "discovered KSU mount registered");
                registered += 1;
            }
            Err(e) => {
                debug!(path = %path, error = %e, "discovered KSU mount registration failed");
                failed += 1;
            }
        }
    }

    if registered > 0 {
        info!(registered, "KSU infra mounts registered with try_umount");
    }

    TryUmountStats { registered, failed }
}
