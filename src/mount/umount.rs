use std::ffi::CString;
use std::path::Path;

use anyhow::{bail, Result};
use tracing::{debug, warn};

/// Unmount an overlay at the given path.
pub fn umount_overlay(target: &Path) -> Result<()> {
    let c_target = CString::new(target.as_os_str().as_encoded_bytes())?;

    let ret = unsafe { libc::umount2(c_target.as_ptr(), libc::MNT_DETACH) };
    if ret != 0 {
        let errno = std::io::Error::last_os_error();
        bail!("umount overlay at {}: {}", target.display(), errno);
    }

    debug!(target = %target.display(), "overlay unmounted");
    Ok(())
}

/// Unmount all bind mounts created for a module.
/// Iterates in reverse order (last mounted = first unmounted).
pub fn umount_magic(mount_paths: &[String]) -> Result<()> {
    for path in mount_paths.iter().rev() {
        let c_path = match CString::new(path.as_bytes()) {
            Ok(p) => p,
            Err(_) => continue,
        };

        let ret = unsafe { libc::umount2(c_path.as_ptr(), libc::MNT_DETACH) };
        if ret != 0 {
            let errno = std::io::Error::last_os_error();
            warn!(path = %path, error = %errno, "magic mount umount failed");
        } else {
            debug!(path = %path, "magic mount unmounted");
        }
    }

    Ok(())
}
