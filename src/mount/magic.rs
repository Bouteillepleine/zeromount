use std::collections::HashSet;
use std::ffi::CString;
use std::fs;
use std::os::unix::fs as unix_fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use tracing::{debug, info, warn};

use crate::core::types::{ModuleFileType, MountResult, MountStrategy, ScannedModule};

fn is_on_readonly_fs(path: &Path) -> bool {
    let check = nearest_existing_ancestor(path);
    let c_path = match CString::new(check.as_os_str().as_encoded_bytes()) {
        Ok(p) => p,
        Err(_) => return false,
    };
    let mut stat = std::mem::MaybeUninit::<libc::statvfs>::uninit();
    let ret = unsafe { libc::statvfs(c_path.as_ptr(), stat.as_mut_ptr()) };
    if ret != 0 {
        return false;
    }
    let stat = unsafe { stat.assume_init() };
    (stat.f_flag & libc::ST_RDONLY) != 0
}

fn nearest_existing_ancestor(path: &Path) -> PathBuf {
    let mut current = path.to_path_buf();
    while !current.exists() {
        match current.parent() {
            Some(p) => current = p.to_path_buf(),
            None => break,
        }
    }
    current
}

fn mount_tmpfs_over(dir: &Path) -> Result<()> {
    let staging = PathBuf::from("/dev/.zm_magic_stage");
    let stage_key = dir.strip_prefix("/").unwrap_or(dir);
    let stage_sub = staging.join(stage_key);
    fs::create_dir_all(&stage_sub)?;

    if dir.is_dir() {
        let _ = copy_dir_recursive(dir, &stage_sub);
    }

    let c_source = CString::new("tmpfs")?;
    let c_target = CString::new(dir.as_os_str().as_encoded_bytes())?;
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
        let _ = fs::remove_dir_all(&staging);
        bail!("tmpfs mount at {} failed: {}", dir.display(), std::io::Error::last_os_error());
    }

    // Restore stock contents + SELinux context
    if stage_sub.is_dir() {
        let _ = copy_dir_recursive(&stage_sub, dir);
        crate::utils::selinux::mirror_selinux_context(&stage_sub, dir);
    }

    let _ = fs::remove_dir_all(&staging);
    debug!(path = %dir.display(), "tmpfs mounted over read-only ancestor");
    Ok(())
}

fn ensure_writable(path: &Path, tmpfs_ancestors: &mut HashSet<PathBuf>) -> Result<()> {
    if !is_on_readonly_fs(path) {
        return Ok(());
    }

    let ancestor = nearest_existing_ancestor(path);

    if tmpfs_ancestors.iter().any(|a| ancestor.starts_with(a)) {
        return Ok(());
    }

    mount_tmpfs_over(&ancestor)?;
    tmpfs_ancestors.insert(ancestor);
    Ok(())
}

/// Magic mount: per-file bind mounts with a tmpfs skeleton for new directories.
///
/// Limitations vs overlay:
/// - No whiteout support (cannot delete stock files)
/// - No opaque directories
/// - Every file creates a visible /proc/mounts entry
/// - Redirect xattrs ignored
///
/// Used when OverlayFS is unavailable (ME03) or as per-module fallback (ME04).
pub fn mount_magic(
    module: &ScannedModule,
    staging_dir: &Path,
) -> Result<MountResult> {
    let mut mount_paths = Vec::new();
    let mut applied = 0u32;
    let mut failed = 0u32;
    let mut errors = Vec::new();
    let mut tmpfs_ancestors = HashSet::new();

    for file in &module.files {
        let source = module.path.join(&file.relative_path);
        let target = PathBuf::from("/").join(&file.relative_path);

        match file.file_type {
            ModuleFileType::Regular | ModuleFileType::Symlink => {
                match bind_mount_file(&source, &target, &mut tmpfs_ancestors) {
                    Ok(()) => {
                        mount_paths.push(target.to_string_lossy().to_string());
                        applied += 1;
                    }
                    Err(e) => {
                        let msg = format!(
                            "bind mount {} -> {}: {e}",
                            source.display(),
                            target.display()
                        );
                        warn!(module = %module.id, "{}", msg);
                        errors.push(msg);
                        failed += 1;
                    }
                }
            }

            ModuleFileType::Directory => {
                // Only bind-mount directories that don't already exist on the target.
                // For existing directories, files inside will be individually mounted.
                if !target.exists() {
                    let skel = staging_dir.join(&module.id).join(&file.relative_path);
                    match create_skeleton_dir(&skel, &source, &target, &mut tmpfs_ancestors) {
                        Ok(()) => {
                            mount_paths.push(target.to_string_lossy().to_string());
                            applied += 1;
                        }
                        Err(e) => {
                            let msg = format!("skeleton dir {}: {e}", target.display());
                            warn!(module = %module.id, "{}", msg);
                            errors.push(msg);
                            failed += 1;
                        }
                    }
                }
            }

            // Magic mount cannot handle whiteouts or opaque dirs
            ModuleFileType::WhiteoutCharDev
            | ModuleFileType::WhiteoutXattr
            | ModuleFileType::WhiteoutAufs
            | ModuleFileType::OpaqueDir => {
                debug!(
                    module = %module.id,
                    path = %file.relative_path.display(),
                    file_type = ?file.file_type,
                    "magic mount cannot handle this file type, skipping"
                );
            }

            // Redirect xattrs: bind-mount the file as-is (no redirect resolution)
            ModuleFileType::RedirectXattr => {
                match bind_mount_file(&source, &target, &mut tmpfs_ancestors) {
                    Ok(()) => {
                        mount_paths.push(target.to_string_lossy().to_string());
                        applied += 1;
                    }
                    Err(e) => {
                        let msg = format!("bind mount redirect {}: {e}", target.display());
                        warn!(module = %module.id, "{}", msg);
                        errors.push(msg);
                        failed += 1;
                    }
                }
            }
        }
    }

    let success = failed == 0 && applied > 0;
    let error = if errors.is_empty() {
        None
    } else {
        Some(errors.join("; "))
    };

    if applied > 0 {
        info!(
            module = %module.id,
            applied,
            failed,
            "magic mount complete"
        );
    }

    Ok(MountResult {
        module_id: module.id.clone(),
        strategy_used: MountStrategy::MagicMount,
        success,
        rules_applied: applied,
        rules_failed: failed,
        error,
        mount_paths,
    })
}

fn bind_mount_file(source: &Path, target: &Path, tmpfs_ancestors: &mut HashSet<PathBuf>) -> Result<()> {
    if let Some(parent) = target.parent() {
        if !parent.exists() {
            if fs::create_dir_all(parent).is_err() {
                ensure_writable(parent, tmpfs_ancestors)?;
                fs::create_dir_all(parent)
                    .with_context(|| format!("cannot create parent: {}", parent.display()))?;
            }
        }
    }

    if !target.exists() {
        if source.is_dir() {
            if fs::create_dir_all(target).is_err() {
                ensure_writable(target, tmpfs_ancestors)?;
                fs::create_dir_all(target)?;
            }
        } else {
            if fs::File::create(target).is_err() {
                ensure_writable(target, tmpfs_ancestors)?;
                fs::File::create(target)
                    .with_context(|| format!("cannot create mount point: {}", target.display()))?;
            }
        }
    }

    let c_source = CString::new(source.as_os_str().as_encoded_bytes())?;
    let c_target = CString::new(target.as_os_str().as_encoded_bytes())?;

    let ret = unsafe {
        libc::mount(
            c_source.as_ptr(),
            c_target.as_ptr(),
            std::ptr::null(),
            libc::MS_BIND,
            std::ptr::null(),
        )
    };

    if ret != 0 {
        bail!(
            "bind mount failed: {}",
            std::io::Error::last_os_error()
        );
    }

    debug!(
        source = %source.display(),
        target = %target.display(),
        "bind mounted"
    );

    Ok(())
}

fn create_skeleton_dir(skeleton: &Path, source: &Path, target: &Path, tmpfs_ancestors: &mut HashSet<PathBuf>) -> Result<()> {
    fs::create_dir_all(skeleton)
        .with_context(|| format!("cannot create skeleton: {}", skeleton.display()))?;

    copy_dir_recursive(source, skeleton)?;

    if let Some(parent) = target.parent() {
        if !parent.exists() {
            if fs::create_dir_all(parent).is_err() {
                ensure_writable(parent, tmpfs_ancestors)?;
                fs::create_dir_all(parent)?;
            }
        }
    }

    if !target.exists() {
        if fs::create_dir_all(target).is_err() {
            ensure_writable(target, tmpfs_ancestors)?;
            fs::create_dir_all(target)?;
        }
    }

    // Bind mount the skeleton to the target
    let c_source = CString::new(skeleton.as_os_str().as_encoded_bytes())?;
    let c_target = CString::new(target.as_os_str().as_encoded_bytes())?;

    let ret = unsafe {
        libc::mount(
            c_source.as_ptr(),
            c_target.as_ptr(),
            std::ptr::null(),
            libc::MS_BIND | libc::MS_REC,
            std::ptr::null(),
        )
    };

    if ret != 0 {
        bail!(
            "bind mount skeleton dir failed: {}",
            std::io::Error::last_os_error()
        );
    }

    Ok(())
}

/// Recursively copy directory contents. Preserves symlinks.
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    let entries = fs::read_dir(src)
        .with_context(|| format!("cannot read dir: {}", src.display()))?;

    for entry in entries.flatten() {
        let src_path = entry.path();
        let file_name = match src_path.file_name() {
            Some(n) => n,
            None => continue,
        };
        let dst_path = dst.join(file_name);

        let metadata = fs::symlink_metadata(&src_path)?;

        if metadata.is_symlink() {
            let link_target = fs::read_link(&src_path)?;
            unix_fs::symlink(&link_target, &dst_path)
                .with_context(|| format!("symlink {} -> {}", dst_path.display(), link_target.display()))?;
        } else if metadata.is_dir() {
            fs::create_dir_all(&dst_path)?;
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)
                .with_context(|| format!("copy {} -> {}", src_path.display(), dst_path.display()))?;
        }
    }

    Ok(())
}
