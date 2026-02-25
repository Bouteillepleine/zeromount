use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use tracing::{debug, warn};

/// Walk a module's system directory looking for `.replace` markers.
/// For each directory with a `.replace` file, set `trusted.overlay.opaque=y`
/// on the corresponding directory in the staging lower dir.
pub fn mark_opaque_dirs(module_system_dir: &Path, lower_dir: &Path) -> Result<()> {
    mark_opaque_recursive(module_system_dir, module_system_dir, lower_dir)
}

fn mark_opaque_recursive(base: &Path, current: &Path, lower_dir: &Path) -> Result<()> {
    let entries = match fs::read_dir(current) {
        Ok(entries) => entries,
        Err(_) => return Ok(()),
    };

    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if path.join(".replace").exists() {
            if let Ok(rel) = path.strip_prefix(base) {
                let staging_dir = lower_dir.join(rel);
                if staging_dir.is_dir() {
                    if let Err(e) = set_opaque_xattr(&staging_dir) {
                        warn!(
                            dir = %staging_dir.display(),
                            error = %e,
                            "failed to set overlay.opaque"
                        );
                    } else {
                        debug!(dir = %staging_dir.display(), "set overlay.opaque");
                    }
                }
            }
        }
        mark_opaque_recursive(base, &path, lower_dir)?;
    }

    Ok(())
}

// overlayfs respects trusted.overlay.opaque ONLY on the upperdir, not on lowerdirs.
// For lowerdir-only (read-only) overlay, the correct mechanism is the whiteout
// marker file .wh..wh..opq inside the directory, which overlayfs recognises in
// any lower layer (fs/overlayfs/namei.c ovl_lookup_index checks for this).
fn set_opaque_xattr(dir: &Path) -> Result<()> {
    let marker = dir.join(".wh..wh..opq");
    fs::File::create(&marker)
        .with_context(|| format!("create opaque marker {}", marker.display()))?;
    Ok(())
}
