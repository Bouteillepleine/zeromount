use std::path::{Path, PathBuf};

use anyhow::Result;
use tracing::{debug, warn};

use crate::core::types::CapabilityFlags;
use crate::susfs::SusfsClient;
use crate::utils::platform;

/// SUSFS module directory names (checked under /data/adb/modules/)
const SUSFS_MODULE_IDS: &[&str] = &["susfs4ksu", "susfs"];

/// DET03: Kernel-first SUSFS probe with stale marker recovery.
///
/// Probes kernel FIRST (ground truth), then reconciles module disable markers.
/// If kernel has SUSFS but module has a stale disable marker from a previous
/// kernel, the marker is removed automatically.
pub fn probe_susfs() -> Result<CapabilityFlags> {
    let mut caps = CapabilityFlags::default();

    let binary = find_susfs_binary();
    match &binary {
        Some(p) => debug!("SUSFS binary found at: {}", p.display()),
        None => debug!("SUSFS binary not found (kernel probe still proceeds)"),
    }

    // Kernel probe FIRST — ground truth
    let kernel_has_susfs = match SusfsClient::probe() {
        Ok(client) if client.is_available() => {
            caps.susfs_available = true;
            caps.susfs_version = client.version().map(String::from);

            let features = client.features();
            caps.susfs_kstat = features.kstat;
            caps.susfs_path = features.path;
            caps.susfs_maps = features.maps;
            caps.susfs_open_redirect = features.open_redirect;
            caps.susfs_kstat_redirect = features.kstat_redirect;
            caps.susfs_open_redirect_all = features.open_redirect_all;

            debug!(
                "SUSFS capabilities: kstat={}, path={}, maps={}, redirect={}, \
                 kstat_redirect={}, redirect_all={}",
                caps.susfs_kstat, caps.susfs_path, caps.susfs_maps,
                caps.susfs_open_redirect, caps.susfs_kstat_redirect,
                caps.susfs_open_redirect_all
            );
            true
        }
        Ok(_) => {
            debug!("SUSFS kernel supercall not responding");
            false
        }
        Err(e) => {
            warn!("SUSFS probe failed: {e}");
            false
        }
    };

    // Reconcile module disable marker with kernel reality
    if kernel_has_susfs && is_susfs_module_disabled() {
        warn!("SUSFS kernel present but module disabled — removing stale marker");
        remove_susfs_disable_marker();
    } else if !kernel_has_susfs && is_susfs_module_disabled() {
        debug!("no SUSFS kernel, disable marker is correct state");
    }

    Ok(caps)
}

/// Check whether SUSFS module directory has a .disabled marker.
/// DET03 layer 1: if disabled, all SUSFS operations are skipped.
fn is_susfs_module_disabled() -> bool {
    let modules_dir = Path::new("/data/adb/modules");
    for module_id in SUSFS_MODULE_IDS {
        let module_dir = modules_dir.join(module_id);
        if module_dir.exists() {
            let disabled = module_dir.join("disable");
            if disabled.exists() {
                debug!("SUSFS module {module_id} has 'disable' marker");
                return true;
            }
            // Module dir exists but not disabled -- proceed
            return false;
        }
    }
    // No SUSFS module dir found -- not disabled (may still have binary)
    false
}

fn remove_susfs_disable_marker() {
    let modules_dir = Path::new("/data/adb/modules");
    for module_id in SUSFS_MODULE_IDS {
        let disable_path = modules_dir.join(module_id).join("disable");
        if disable_path.exists() {
            if let Err(_e) = std::fs::remove_file(&disable_path) {
                warn!("failed to remove stale disable marker: {}", disable_path.display());
            } else {
                debug!("removed stale disable marker: {}", disable_path.display());
            }
        }
    }
}

/// Locate the SUSFS binary by searching platform-specific paths.
/// DET03 layer 2: search order per RootManager::susfs_binary_paths().
pub fn find_susfs_binary() -> Option<PathBuf> {
    // Try platform-specific paths first
    if let Ok(manager) = platform::detect_root_manager() {
        for path in manager.susfs_binary_paths() {
            if path.exists() && is_executable(&path) {
                return Some(path);
            }
        }
    }

    // Fallback: check common paths
    let fallback_paths = [
        "/data/adb/ksu/bin/ksu_susfs",
        "/data/adb/ap/bin/ksu_susfs",
        "/data/adb/ksu/bin/susfs",
        "/data/adb/modules/meta-zeromount/ksu_susfs",
    ];

    for path in &fallback_paths {
        let p = Path::new(path);
        if p.exists() && is_executable(p) {
            return Some(p.to_path_buf());
        }
    }

    None
}

fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .map(|m| m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}
