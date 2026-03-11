mod enforcer;
mod table;

use std::fs;

use anyhow::Result;
use resetprop::PropSystem;
use tracing::{info, warn};

use crate::core::config::ZeroMountConfig;

const EXTERNAL_SUSFS_FLAG: &str = "/data/adb/zeromount/flags/external_susfs";

pub fn run_prop_watch() -> Result<()> {
    let config = ZeroMountConfig::load(None)?;

    if !config.brene.prop_spoofing {
        info!("prop-watch: prop spoofing disabled, exiting");
        return Ok(());
    }

    if has_external_susfs() {
        info!("prop spoofing deferred to external module");
        return Ok(());
    }

    let sys = match PropSystem::open() {
        Ok(s) => s,
        Err(e) => {
            warn!("prop-watch: cannot open property areas: {e}");
            return Ok(());
        }
    };

    let props: Vec<(&str, &str)> = table::GENERAL
        .iter()
        .map(|p| (p.name, p.value))
        .collect();
    enforcer::enforce_once(&sys, &props);

    let size = config.brene.vbmeta_size.to_string();
    let _ = sys.set("ro.boot.vbmeta.size", &size);

    if !config.brene.verified_boot_hash.is_empty() {
        let _ = sys.set("ro.boot.vbmeta.digest", &config.brene.verified_boot_hash);
    }

    info!("general prop spoofing applied");
    Ok(())
}

fn has_external_susfs() -> bool {
    fs::read_to_string(EXTERNAL_SUSFS_FLAG)
        .map(|s| {
            let v = s.trim();
            !v.is_empty() && v != "none"
        })
        .unwrap_or(false)
}
