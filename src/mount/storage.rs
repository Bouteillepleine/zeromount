use std::ffi::CString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};
use tracing::{debug, info, warn};

use crate::core::config::{MountConfig, StorageMode as ConfigStorageMode};
use crate::core::types::CapabilityFlags;

const MOUNT_SOURCE: &str = "KSU";
const RANDOM_PATH_LEN: usize = 12;
const FIXED_PATH_NAME: &str = "zeromount";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageMode {
    Erofs,
    Tmpfs,
    Ext4,
}

/// Handle to a prepared storage area. Drop-safe: cleanup on drop.
#[derive(Debug)]
pub struct StorageHandle {
    pub mode: StorageMode,
    pub base_path: PathBuf,
    /// Per-module lower directories live under base_path/<module_id>/
    cleaned_up: bool,
}

impl StorageHandle {
    /// Get the lower directory path for a specific module's partition content.
    pub fn lower_dir(&self, module_id: &str, partition: &str) -> PathBuf {
        self.base_path.join(module_id).join(partition)
    }

    /// Get the work directory for overlay mounts.
    pub fn work_dir(&self, mount_point: &str) -> PathBuf {
        // Replace / with _ for the work dir name
        let safe_name = mount_point.replace('/', "_");
        self.base_path.join(".work").join(safe_name)
    }

    /// Get the upper directory for overlay mounts.
    #[allow(dead_code)] // API for overlay upper layer path
    pub fn upper_dir(&self, mount_point: &str) -> PathBuf {
        let safe_name = mount_point.replace('/', "_");
        self.base_path.join(".upper").join(safe_name)
    }
}

impl Drop for StorageHandle {
    fn drop(&mut self) {
        if !self.cleaned_up {
            if let Err(e) = cleanup_storage_inner(&self.base_path, self.mode) {
                warn!(error = %e, "storage cleanup failed during drop");
            }
        }
    }
}

/// ME01: Initialize storage respecting config preference, falling back via cascade.
/// ME11: Random or fixed mount path under /mnt/.
pub fn init_storage(capabilities: &CapabilityFlags, mount_config: &MountConfig) -> Result<StorageHandle> {
    let base_path = if mount_config.random_mount_paths {
        generate_random_path()
    } else {
        generate_fixed_path()
    };

    info!(path = %base_path.display(), random = mount_config.random_mount_paths, "staging path selected");

    fs::create_dir_all(&base_path)
        .with_context(|| format!("cannot create staging dir: {}", base_path.display()))?;

    // If user forced a specific mode, try it first
    match mount_config.storage_mode {
        ConfigStorageMode::Erofs => {
            if let Some(handle) = try_mode_erofs(&base_path, capabilities) {
                return Ok(handle);
            }
            warn!("forced EROFS failed, falling back to cascade");
        }
        ConfigStorageMode::Tmpfs => {
            if let Some(handle) = try_mode_tmpfs(&base_path, capabilities) {
                return Ok(handle);
            }
            warn!("forced tmpfs failed, falling back to cascade");
        }
        ConfigStorageMode::Ext4 => {
            if let Some(handle) = try_mode_ext4(&base_path) {
                return Ok(handle);
            }
            warn!("forced ext4 failed, falling back to cascade");
        }
        ConfigStorageMode::Auto => {}
    }

    // Cascade: EROFS -> tmpfs+xattr -> ext4 -> bare tmpfs
    if let Some(handle) = try_mode_erofs(&base_path, capabilities) {
        return Ok(handle);
    }
    if let Some(handle) = try_mode_tmpfs(&base_path, capabilities) {
        return Ok(handle);
    }
    if let Some(handle) = try_mode_ext4(&base_path) {
        return Ok(handle);
    }

    // Bare tmpfs fallback (no xattr guarantee)
    match mount_tmpfs_at(&base_path) {
        Ok(()) => {
            info!(mode = "tmpfs", path = %base_path.display(), "storage initialized (bare fallback)");
        }
        Err(e) => {
            warn!(error = %e, "all storage mounts failed, using bare directory");
        }
    }
    Ok(StorageHandle {
        mode: StorageMode::Tmpfs,
        base_path,
        cleaned_up: false,
    })
}

fn try_mode_erofs(base_path: &Path, capabilities: &CapabilityFlags) -> Option<StorageHandle> {
    if !capabilities.erofs_supported || !is_erofs_available() {
        return None;
    }
    match try_erofs_storage(base_path) {
        Ok(()) => {
            info!(mode = "erofs", path = %base_path.display(), "storage initialized");
            Some(StorageHandle { mode: StorageMode::Erofs, base_path: base_path.to_path_buf(), cleaned_up: false })
        }
        Err(e) => {
            debug!(error = %e, "EROFS init failed");
            let _ = do_umount(base_path);
            None
        }
    }
}

fn try_mode_tmpfs(base_path: &Path, capabilities: &CapabilityFlags) -> Option<StorageHandle> {
    if !capabilities.tmpfs_xattr {
        return None;
    }
    match try_tmpfs_with_xattr(base_path) {
        Ok(()) => {
            info!(mode = "tmpfs", path = %base_path.display(), "storage initialized");
            Some(StorageHandle { mode: StorageMode::Tmpfs, base_path: base_path.to_path_buf(), cleaned_up: false })
        }
        Err(e) => {
            debug!(error = %e, "tmpfs with xattr failed");
            let _ = do_umount(base_path);
            None
        }
    }
}

fn try_mode_ext4(base_path: &Path) -> Option<StorageHandle> {
    match try_ext4_storage(base_path) {
        Ok(()) => {
            info!(mode = "ext4", path = %base_path.display(), "storage initialized");
            Some(StorageHandle { mode: StorageMode::Ext4, base_path: base_path.to_path_buf(), cleaned_up: false })
        }
        Err(e) => {
            debug!(error = %e, "ext4 loopback failed");
            None
        }
    }
}

/// Explicitly clean up storage. Preferred over relying on Drop.
pub fn cleanup_storage(handle: &mut StorageHandle) -> Result<()> {
    cleanup_storage_inner(&handle.base_path, handle.mode)?;
    handle.cleaned_up = true;
    Ok(())
}

fn cleanup_storage_inner(base_path: &Path, _mode: StorageMode) -> Result<()> {
    // Attempt unmount first (may fail if not mounted, that's fine)
    let _ = do_umount(base_path);

    // Remove the directory tree
    if base_path.exists() {
        fs::remove_dir_all(base_path)
            .with_context(|| format!("cannot remove staging dir: {}", base_path.display()))?;
    }

    Ok(())
}

/// ME11: Generate random 12-char alphanumeric path under /mnt/.
/// Falls back to /mnt/vendor/ if /mnt/ is not writable.
fn generate_random_path() -> PathBuf {
    resolve_mount_base(&random_alphanum(RANDOM_PATH_LEN))
}

fn generate_fixed_path() -> PathBuf {
    resolve_mount_base(FIXED_PATH_NAME)
}

fn resolve_mount_base(name: &str) -> PathBuf {
    if is_dir_writable("/mnt") {
        return PathBuf::from("/mnt").join(name);
    }
    if is_dir_writable("/mnt/vendor") {
        return PathBuf::from("/mnt/vendor").join(name);
    }
    PathBuf::from("/dev").join(name)
}

fn random_alphanum(len: usize) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0xDEAD_BEEF);

    // Simple LCG seeded from time -- sufficient for path randomization
    let mut state = seed as u64;
    let chars: Vec<u8> = (0..len)
        .map(|_| {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let idx = ((state >> 33) % 36) as u8;
            if idx < 10 {
                b'0' + idx
            } else {
                b'a' + (idx - 10)
            }
        })
        .collect();

    String::from_utf8(chars).unwrap_or_else(|_| "zeromount_tmp".to_string())
}

fn is_dir_writable(path: &str) -> bool {
    let c_path = match CString::new(path) {
        Ok(p) => p,
        Err(_) => return false,
    };
    unsafe { libc::access(c_path.as_ptr(), libc::W_OK) == 0 }
}

/// Check if the kernel supports EROFS by reading /proc/filesystems.
fn is_erofs_available() -> bool {
    fs::read_to_string("/proc/filesystems")
        .map(|content| content.lines().any(|l| l.contains("erofs")))
        .unwrap_or(false)
}

/// Create EROFS image from base_path content, mount it read-only, then nuke the image.
fn try_erofs_storage(base_path: &Path) -> Result<()> {
    let image_path = base_path.with_extension("erofs.img");

    let status = Command::new("mkfs.erofs")
        .args(["-z", "lz4hc", "-x", "256"])
        .arg(&image_path)
        .arg(base_path)
        .output()
        .context("mkfs.erofs not found")?;

    if !status.status.success() {
        let stderr = String::from_utf8_lossy(&status.stderr);
        bail!("mkfs.erofs failed: {stderr}");
    }

    // Mount EROFS image read-only
    let c_source = CString::new(image_path.as_os_str().as_encoded_bytes())?;
    let c_target = CString::new(base_path.as_os_str().as_encoded_bytes())?;
    let c_fstype = CString::new("erofs")?;

    let ret = unsafe {
        libc::mount(
            c_source.as_ptr(),
            c_target.as_ptr(),
            c_fstype.as_ptr(),
            libc::MS_RDONLY,
            std::ptr::null(),
        )
    };

    if ret != 0 {
        let errno = std::io::Error::last_os_error();
        let _ = fs::remove_file(&image_path);
        bail!("mount erofs at {}: {}", base_path.display(), errno);
    }

    // ME12: nuke image after mount — kernel keeps inode alive
    let _ = nuke_backing_file(&image_path);
    Ok(())
}

/// Mount tmpfs and verify xattr support for overlay whiteouts.
fn try_tmpfs_with_xattr(base_path: &Path) -> Result<()> {
    mount_tmpfs_at(base_path)?;

    // Test xattr support: overlay needs trusted.overlay.whiteout
    let test_path = base_path.join(".xattr_test");
    let _ = fs::write(&test_path, "");

    let c_path = CString::new(test_path.as_os_str().as_encoded_bytes())?;
    let c_name = CString::new("trusted.overlay.whiteout")?;
    let value = b"y";

    let ret = unsafe {
        libc::setxattr(
            c_path.as_ptr(),
            c_name.as_ptr(),
            value.as_ptr() as *const libc::c_void,
            value.len(),
            0,
        )
    };

    let _ = fs::remove_file(&test_path);

    if ret != 0 {
        bail!("tmpfs lacks xattr support for overlay whiteouts");
    }

    Ok(())
}

/// Create a sparse ext4 image and loop-mount it.
fn try_ext4_storage(base_path: &Path) -> Result<()> {
    let image_path = base_path.with_extension("ext4.img");

    // Sparse image: 2GB virtual, near-zero actual disk usage
    let dd_status = Command::new("dd")
        .args([
            "if=/dev/zero",
            &format!("of={}", image_path.display()),
            "bs=1M",
            "count=0",
            "seek=2048",
        ])
        .output()
        .context("dd not found")?;

    if !dd_status.status.success() {
        let stderr = String::from_utf8_lossy(&dd_status.stderr);
        bail!("dd sparse image failed: {stderr}");
    }

    // Format without journal to reduce overhead
    let mkfs_status = Command::new("mkfs.ext4")
        .args(["-O", "^has_journal"])
        .arg(&image_path)
        .output()
        .context("mkfs.ext4 not found")?;

    if !mkfs_status.status.success() {
        let _ = fs::remove_file(&image_path);
        let stderr = String::from_utf8_lossy(&mkfs_status.stderr);
        bail!("mkfs.ext4 failed: {stderr}");
    }

    // Loop mount with noatime
    let c_source = CString::new(image_path.as_os_str().as_encoded_bytes())?;
    let c_target = CString::new(base_path.as_os_str().as_encoded_bytes())?;
    let c_fstype = CString::new("ext4")?;
    let c_data = CString::new("loop")?;

    let ret = unsafe {
        libc::mount(
            c_source.as_ptr(),
            c_target.as_ptr(),
            c_fstype.as_ptr(),
            libc::MS_NOATIME,
            c_data.as_ptr() as *const libc::c_void,
        )
    };

    if ret != 0 {
        let errno = std::io::Error::last_os_error();
        let _ = fs::remove_file(&image_path);
        bail!("mount ext4 at {}: {}", base_path.display(), errno);
    }

    Ok(())
}

/// Mount tmpfs at target with source name "KSU" (ME09).
fn mount_tmpfs_at(target: &Path) -> Result<()> {
    let c_source = CString::new(MOUNT_SOURCE)?;
    let c_target = CString::new(target.as_os_str().as_encoded_bytes())?;
    let c_fstype = CString::new("tmpfs")?;
    let c_data = CString::new("mode=0755")?;

    let ret = unsafe {
        libc::mount(
            c_source.as_ptr(),
            c_target.as_ptr(),
            c_fstype.as_ptr(),
            0,
            c_data.as_ptr() as *const libc::c_void,
        )
    };

    if ret != 0 {
        let errno = std::io::Error::last_os_error();
        bail!("mount tmpfs at {}: {}", target.display(), errno);
    }

    Ok(())
}

/// Unmount a path. Returns Ok even if not mounted.
fn do_umount(target: &Path) -> Result<()> {
    let c_target = match CString::new(target.as_os_str().as_encoded_bytes()) {
        Ok(p) => p,
        Err(_) => return Ok(()),
    };

    let ret = unsafe { libc::umount2(c_target.as_ptr(), libc::MNT_DETACH) };
    if ret != 0 {
        let errno = std::io::Error::last_os_error();
        // EINVAL = not mounted, which is fine
        if errno.raw_os_error() != Some(libc::EINVAL) {
            debug!(path = %target.display(), error = %errno, "umount failed");
        }
    }

    Ok(())
}

/// ME12: Delete a backing file after mount. The kernel keeps the inode alive
/// via the mount reference, but the file disappears from the directory.
pub fn nuke_backing_file(path: &Path) -> Result<()> {
    if path.exists() {
        fs::remove_file(path)
            .with_context(|| format!("cannot nuke backing file: {}", path.display()))?;
        debug!(path = %path.display(), "nuked backing file");
    }
    Ok(())
}
