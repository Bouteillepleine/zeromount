use tracing::{debug, info};

pub struct StockOverlay {
    pub mount_point: String,
    #[allow(dead_code)] // reserved for Option B overlay absorption
    pub peer_group_id: u32,
}

pub fn collect_stock_overlays() -> Vec<StockOverlay> {
    let content = match std::fs::read_to_string("/proc/self/mountinfo") {
        Ok(c) => c,
        Err(e) => {
            info!(error = %e, "mountinfo read failed, skipping stock overlay collection");
            return Vec::new();
        }
    };

    let overlay_count = content.lines().filter(|l| l.contains("overlay")).count();
    debug!(lines = content.lines().count(), overlays = overlay_count, "mountinfo snapshot for stock overlay scan");

    let mut results = Vec::new();
    for line in content.lines() {
        if let Some(entry) = parse_overlay_mount(line) {
            debug!(
                path = %entry.mount_point,
                peer_group = entry.peer_group_id,
                "stock OEM overlay found"
            );
            results.push(entry);
        }
    }

    if results.is_empty() {
        debug!("no stock OEM overlays found in mountinfo");
    } else {
        info!(count = results.len(), "stock OEM overlays collected");
    }
    results
}

fn parse_overlay_mount(line: &str) -> Option<StockOverlay> {
    let fields: Vec<&str> = line.split_whitespace().collect();
    let sep = fields.iter().position(|&f| f == "-")?;

    let fstype = *fields.get(sep + 1)?;
    if fstype != "overlay" {
        return None;
    }

    let mount_point = *fields.get(4)?;
    let super_opts = *fields.get(sep + 3)?;

    if !has_oem_lowerdir(super_opts) {
        debug!(path = mount_point, "overlay skipped: not OEM lowerdir pattern");
        return None;
    }

    let peer_group_id = fields[5..sep]
        .iter()
        .find_map(|f| f.strip_prefix("shared:"))
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    Some(StockOverlay {
        mount_point: mount_point.to_string(),
        peer_group_id,
    })
}

// OEM overlay partitions injected during Android init
fn has_oem_lowerdir(super_opts: &str) -> bool {
    for opt in super_opts.split(',') {
        if let Some(lowerdir) = opt.strip_prefix("lowerdir=") {
            return lowerdir.contains("/mi_ext/")
                || lowerdir.contains("/prism/")
                || lowerdir.contains("/optics/")
                || lowerdir.contains("/my_");
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_xiaomi_mi_ext() {
        let line = "95 49 0:34 / /product/overlay ro,relatime shared:26 - overlay overlay ro,seclabel,lowerdir=/mnt/vendor/mi_ext/product/overlay/:/product/overlay";
        let entry = parse_overlay_mount(line).unwrap();
        assert_eq!(entry.mount_point, "/product/overlay");
        assert_eq!(entry.peer_group_id, 26);
    }

    #[test]
    fn parse_samsung_prism() {
        let line = "100 49 0:40 / /system/priv-app ro,relatime shared:30 - overlay overlay ro,seclabel,lowerdir=/prism/system/priv-app:/system/priv-app";
        let entry = parse_overlay_mount(line).unwrap();
        assert_eq!(entry.mount_point, "/system/priv-app");
        assert_eq!(entry.peer_group_id, 30);
    }

    #[test]
    fn parse_oppo_my_product() {
        let line = "110 49 0:50 / /product/app ro,relatime shared:35 - overlay overlay ro,seclabel,lowerdir=/my_product/app:/product/app";
        let entry = parse_overlay_mount(line).unwrap();
        assert_eq!(entry.mount_point, "/product/app");
        assert_eq!(entry.peer_group_id, 35);
    }

    #[test]
    fn skip_zeromount_overlay() {
        let line = "151 36 0:89 / /system/bin rw,relatime shared:40 - overlay KSU ro,seclabel,lowerdir=/mnt/abc123/clean/system/bin:/system/bin";
        assert!(parse_overlay_mount(line).is_none());
    }

    #[test]
    fn skip_non_overlay() {
        let line = "36 35 253:5 / / ro,relatime shared:1 - erofs /dev/block/dm-5 ro,seclabel";
        assert!(parse_overlay_mount(line).is_none());
    }

    #[test]
    fn oem_pattern_mi_ext() {
        assert!(has_oem_lowerdir("ro,seclabel,lowerdir=/mnt/vendor/mi_ext/product/app/:/product/app"));
    }

    #[test]
    fn oem_pattern_prism() {
        assert!(has_oem_lowerdir("ro,seclabel,lowerdir=/prism/system/app:/system/app"));
    }

    #[test]
    fn oem_pattern_optics() {
        assert!(has_oem_lowerdir("ro,seclabel,lowerdir=/optics/overlay:/product/overlay"));
    }

    #[test]
    fn oem_pattern_oppo_my() {
        assert!(has_oem_lowerdir("ro,seclabel,lowerdir=/my_product/app:/product/app"));
    }

    #[test]
    fn non_oem_pattern() {
        assert!(!has_oem_lowerdir("ro,seclabel,lowerdir=/mnt/abc123/clean/system/bin:/system/bin"));
    }
}
