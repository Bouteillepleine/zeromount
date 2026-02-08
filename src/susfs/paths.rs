use std::path::Path;

use anyhow::{bail, Result};
use tracing::debug;

use super::SusfsClient;

/// Hide a list of paths, skipping any that don't exist.
/// Returns the count of successfully hidden paths.
pub fn hide_paths(client: &SusfsClient, paths: &[&str]) -> Result<u32> {
    if !client.is_available() || !client.features().path {
        bail!("SUSFS path hiding not available");
    }

    let mut count = 0u32;
    for path in paths {
        if !Path::new(path).exists() {
            debug!("skip nonexistent path: {path}");
            continue;
        }
        match client.add_sus_path(path) {
            Ok(()) => count += 1,
            Err(e) => debug!("add_sus_path failed for {path}: {e}"),
        }
    }
    Ok(count)
}

/// Hide a list of paths with re-flag per zygote spawn.
pub fn hide_paths_loop(client: &SusfsClient, paths: &[&str]) -> Result<u32> {
    if !client.is_available() || !client.features().path {
        bail!("SUSFS path hiding not available");
    }

    let mut count = 0u32;
    for path in paths {
        if !Path::new(path).exists() {
            debug!("skip nonexistent path: {path}");
            continue;
        }
        match client.add_sus_path_loop(path) {
            Ok(()) => count += 1,
            Err(e) => debug!("add_sus_path_loop failed for {path}: {e}"),
        }
    }
    Ok(count)
}

/// Hide a list of library paths from /proc/self/maps.
pub fn hide_maps(client: &SusfsClient, map_paths: &[&str]) -> Result<u32> {
    if !client.is_available() || !client.features().maps {
        bail!("SUSFS maps hiding not available");
    }

    let mut count = 0u32;
    for path in map_paths {
        match client.add_sus_map(path) {
            Ok(()) => count += 1,
            Err(e) => debug!("add_sus_map failed for {path}: {e}"),
        }
    }
    Ok(count)
}
