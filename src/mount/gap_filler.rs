use std::collections::BTreeSet;
use std::ffi::CString;
use std::fs;
use std::path::PathBuf;

use anyhow::{bail, Result};
use tracing::{debug, info};

const MAX_FILLERS: usize = 32;

// Xiaomi (and other OEMs) create mi_ext overlay mounts during init for empty
// directories, then remove them. The freed peer group IDs leave gaps in the
// shared:N sequence visible in /proc/self/mountinfo. Detector apps flag these
// as evidence of mount tampering. The kernel's IDR allocator reuses freed IDs,
// so creating new shared mounts at innocuous paths consumes the gap slots.
pub fn fill_peer_group_gaps() -> u32 {
    let ids = match parse_shared_peer_ids() {
        Ok(ids) => ids,
        Err(e) => {
            debug!(error = %e, "peer group parse failed, skipping gap fill");
            return 0;
        }
    };

    if ids.is_empty() {
        return 0;
    }

    let gaps = find_gaps(&ids);
    if gaps.is_empty() {
        debug!("no peer group gaps found");
        return 0;
    }

    info!(count = gaps.len(), gaps = ?gaps, "filling peer group gaps");

    let mut filled = 0u32;
    for gap_id in &gaps {
        let path = PathBuf::from(format!("/mnt/.gc{:x}", gap_id));
        if let Err(e) = create_shared_filler(&path) {
            debug!(id = gap_id, error = %e, "filler mount failed");
            continue;
        }
        filled += 1;
    }

    if filled > 0 {
        info!(filled, "peer group gaps filled");
    }
    filled
}

fn parse_shared_peer_ids() -> Result<BTreeSet<u32>> {
    let content = fs::read_to_string("/proc/self/mountinfo")?;
    let mut ids = BTreeSet::new();

    for line in content.lines() {
        for field in line.split_whitespace() {
            if let Some(id_str) = field.strip_prefix("shared:") {
                if let Ok(id) = id_str.parse::<u32>() {
                    ids.insert(id);
                }
            }
        }
    }

    Ok(ids)
}

// Only fill gaps within the main contiguous block — skip large jumps
// between mount groups (e.g., 63 → 773 for FUSE mounts).
fn find_gaps(ids: &BTreeSet<u32>) -> Vec<u32> {
    let sorted: Vec<u32> = ids.iter().copied().collect();
    let mut gaps = Vec::new();

    for window in sorted.windows(2) {
        let (lo, hi) = (window[0], window[1]);
        let span = hi - lo;
        if span > 1 && span < 64 {
            for id in (lo + 1)..hi {
                gaps.push(id);
                if gaps.len() >= MAX_FILLERS {
                    return gaps;
                }
            }
        }
    }

    gaps
}

// Mount under /mnt (shared:7) so the new tmpfs inherits shared propagation
// and gets allocated a reused peer group ID from the freed pool.
fn create_shared_filler(path: &PathBuf) -> Result<()> {
    fs::create_dir_all(path)?;

    let c_source = CString::new("none")?;
    let c_target = CString::new(path.as_os_str().as_encoded_bytes())?;
    let c_fstype = CString::new("tmpfs")?;
    let c_data = CString::new("size=0,nr_inodes=1,mode=000")?;

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
        let _ = fs::remove_dir(path);
        bail!("filler tmpfs at {}: {}", path.display(), std::io::Error::last_os_error());
    }

    debug!(path = %path.display(), "filler mount created");
    Ok(())
}

/// Bind-mount each try_umount-registered path to a hidden location so the
/// peer group ID survives in app namespaces after try_umount removes the
/// original. Without this, magic mount's MS_MOVE destroys stock overlays
/// and try_umount then removes our tmpfs — the peer group vanishes entirely.
pub fn preserve_mount_peer_groups(mount_paths: &[String]) -> u32 {
    let content = match fs::read_to_string("/proc/self/mountinfo") {
        Ok(c) => c,
        Err(e) => {
            debug!(error = %e, "mountinfo read failed for peer group preservation");
            return 0;
        }
    };

    let shadow_base = PathBuf::from("/mnt/.pg");
    if let Err(e) = fs::create_dir_all(&shadow_base) {
        debug!(error = %e, "failed to create shadow mount base");
        return 0;
    }

    let mut preserved = 0u32;
    let mut seen_groups = BTreeSet::new();

    for line in content.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 5 {
            continue;
        }
        let mount_point = fields[4];
        if !mount_paths.iter().any(|p| p == mount_point) {
            continue;
        }

        let peer_group = fields.iter()
            .find_map(|f| f.strip_prefix("shared:"))
            .and_then(|s| s.parse::<u32>().ok());

        let Some(pg) = peer_group else { continue };
        if !seen_groups.insert(pg) {
            continue;
        }

        let shadow_path = shadow_base.join(format!("{pg}"));

        let is_dir = fs::metadata(mount_point).map(|m| m.is_dir()).unwrap_or(true);
        if is_dir {
            if let Err(e) = fs::create_dir_all(&shadow_path) {
                debug!(peer_group = pg, error = %e, "shadow dir creation failed");
                continue;
            }
        } else {
            if let Some(parent) = shadow_path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if let Err(e) = fs::File::create(&shadow_path) {
                debug!(peer_group = pg, error = %e, "shadow file creation failed");
                continue;
            }
        }

        let c_src = match CString::new(mount_point) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let c_dst = match CString::new(shadow_path.as_os_str().as_encoded_bytes()) {
            Ok(s) => s,
            Err(_) => continue,
        };

        let ret = unsafe {
            libc::mount(
                c_src.as_ptr(),
                c_dst.as_ptr(),
                std::ptr::null(),
                libc::MS_BIND,
                std::ptr::null(),
            )
        };

        if ret != 0 {
            let err = std::io::Error::last_os_error();
            debug!(peer_group = pg, path = mount_point, error = %err, "shadow bind mount failed");
            let _ = fs::remove_dir(&shadow_path);
            continue;
        }

        // Read-only to minimize exposure
        unsafe {
            libc::mount(
                std::ptr::null(),
                c_dst.as_ptr(),
                std::ptr::null(),
                libc::MS_REMOUNT | libc::MS_BIND | libc::MS_RDONLY,
                std::ptr::null(),
            );
        }

        debug!(peer_group = pg, path = mount_point, shadow = %shadow_path.display(), "peer group preserved");
        preserved += 1;
    }

    if preserved > 0 {
        info!(preserved, "mount peer groups preserved via shadow bind mounts");
    }
    preserved
}

/// Log peer group IDs assigned to our mount paths for diagnostics.
/// Helps verify that pre-mount gap filling pushed our IDs to end-of-range.
pub fn log_mount_peer_groups(mount_paths: &[String]) {
    let content = match fs::read_to_string("/proc/self/mountinfo") {
        Ok(c) => c,
        Err(e) => {
            debug!(error = %e, "mountinfo read failed for peer group logging");
            return;
        }
    };

    let all_ids = match parse_shared_peer_ids() {
        Ok(ids) => ids,
        Err(_) => BTreeSet::new(),
    };
    let max_id = all_ids.iter().next_back().copied().unwrap_or(0);

    for line in content.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 5 {
            continue;
        }
        let mount_point = fields[4];
        if !mount_paths.iter().any(|p| p == mount_point) {
            continue;
        }
        for field in &fields {
            if let Some(id_str) = field.strip_prefix("shared:") {
                if let Ok(id) = id_str.parse::<u32>() {
                    let position = if id >= max_id - mount_paths.len() as u32 {
                        "end-of-range"
                    } else {
                        "mid-range"
                    };
                    info!(
                        path = mount_point,
                        peer_group = id,
                        max_peer_group = max_id,
                        position,
                        "mount peer group assigned"
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_gaps_contiguous() {
        let ids: BTreeSet<u32> = [1, 2, 3, 4, 5].into();
        assert!(find_gaps(&ids).is_empty());
    }

    #[test]
    fn find_gaps_small_gap() {
        let ids: BTreeSet<u32> = [1, 2, 3, 5, 6].into();
        assert_eq!(find_gaps(&ids), vec![4]);
    }

    #[test]
    fn find_gaps_multiple() {
        let ids: BTreeSet<u32> = [1, 2, 5, 8, 9].into();
        assert_eq!(find_gaps(&ids), vec![3, 4, 6, 7]);
    }

    #[test]
    fn find_gaps_ignores_large_jump() {
        let ids: BTreeSet<u32> = [1, 2, 3, 500, 501].into();
        assert!(find_gaps(&ids).is_empty());
    }

    #[test]
    fn find_gaps_skips_large_jump_but_fills_small() {
        let mut ids: BTreeSet<u32> = (1..=38).collect();
        ids.extend([41, 42, 43, 63, 773, 774]);
        let gaps = find_gaps(&ids);
        assert!(gaps.contains(&39));
        assert!(gaps.contains(&40));
        assert!(!gaps.iter().any(|&g| g > 63));
    }
}
