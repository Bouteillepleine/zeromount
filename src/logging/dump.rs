use std::fs::{self, File, OpenOptions};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use rand::Rng;

const DUMP_BASE: &str = "/sdcard";
const LOCK_PATH: &str = "/data/adb/zeromount/.dump_lock";
const DUMP_PATH_FILE: &str = "/data/adb/zeromount/.dump_path";
const DMESG_SIZE_LIMIT: u64 = 2 * 1024 * 1024;
const TOTAL_SIZE_LIMIT: u64 = 10 * 1024 * 1024;

pub fn execute_dump() -> Result<()> {
    let lock = acquire_flock(LOCK_PATH)?;

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let dir_name = random_name(8);
    let dump_dir = PathBuf::from(DUMP_BASE).join(&dir_name);
    fs::create_dir_all(&dump_dir).context("cannot create dump dir")?;
    set_perms(&dump_dir, 0o755)?;

    let mut manifest_files: Vec<serde_json::Value> = Vec::new();
    let mut total_size: u64 = 0;

    // Rotating log files from log_dir
    let config = crate::core::config::ZeroMountConfig::load(None).unwrap_or_default();
    let log_dir = &config.logging.log_dir;
    copy_log_files(log_dir, &dump_dir, &mut manifest_files, &mut total_size)?;

    // dmesg filtered for zeromount
    let zm_dmesg_name = format!("{}.log", random_name(6));
    let zm_dmesg_path = dump_dir.join(&zm_dmesg_name);
    let zm_bytes = collect_dmesg("zeromount", &zm_dmesg_path)?;
    total_size += zm_bytes;
    manifest_files.push(serde_json::json!({
        "file": zm_dmesg_name,
        "description": "dmesg filtered for zeromount",
        "bytes": zm_bytes,
    }));

    // dmesg filtered for susfs
    let susfs_dmesg_name = format!("{}.log", random_name(6));
    let susfs_dmesg_path = dump_dir.join(&susfs_dmesg_name);
    let susfs_bytes = collect_dmesg("susfs", &susfs_dmesg_path)?;
    total_size += susfs_bytes;
    manifest_files.push(serde_json::json!({
        "file": susfs_dmesg_name,
        "description": "dmesg filtered for susfs",
        "bytes": susfs_bytes,
    }));

    // Config state
    let config_name = format!("{}.txt", random_name(6));
    let config_path = dump_dir.join(&config_name);
    let config_bytes = dump_config(&config, &config_path)?;
    total_size += config_bytes;
    manifest_files.push(serde_json::json!({
        "file": config_name,
        "description": "config state",
        "bytes": config_bytes,
    }));

    // Sysfs debug level
    let sysfs_name = format!("{}.txt", random_name(6));
    let sysfs_path = dump_dir.join(&sysfs_name);
    let sysfs_bytes = dump_sysfs_level(&sysfs_path)?;
    total_size += sysfs_bytes;
    manifest_files.push(serde_json::json!({
        "file": sysfs_name,
        "description": "sysfs debug level",
        "bytes": sysfs_bytes,
    }));

    // SUSFS probe info
    let susfs_name = format!("{}.txt", random_name(6));
    let susfs_path = dump_dir.join(&susfs_name);
    let susfs_probe_bytes = dump_susfs_probe(&susfs_path)?;
    total_size += susfs_probe_bytes;
    manifest_files.push(serde_json::json!({
        "file": susfs_name,
        "description": "susfs probe info",
        "bytes": susfs_probe_bytes,
    }));

    if total_size > TOTAL_SIZE_LIMIT {
        eprintln!("warning: dump total size {} exceeds 10 MB limit", total_size);
    }

    // Manifest written to stdout and persisted as .dat
    let manifest = serde_json::json!({
        "timestamp": timestamp,
        "dump_dir": dump_dir.to_string_lossy(),
        "total_bytes": total_size,
        "files": manifest_files,
    });
    let manifest_str = serde_json::to_string_pretty(&manifest)?;
    println!("{manifest_str}");

    let manifest_name = format!("{}.dat", random_name(6));
    let manifest_path = dump_dir.join(&manifest_name);
    write_file(&manifest_path, manifest_str.as_bytes())?;

    // Write dump path for WebUI discovery
    let dump_dir_str = dump_dir.to_string_lossy().to_string();
    if let Some(parent) = Path::new(DUMP_PATH_FILE).parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(DUMP_PATH_FILE, &dump_dir_str)
        .context("cannot write .dump_path")?;

    drop(lock);
    Ok(())
}

// -- helpers --

fn random_name(len: usize) -> String {
    let mut rng = rand::thread_rng();
    (0..len)
        .map(|_| {
            let idx = rng.gen_range(0..36u8);
            (if idx < 10 { b'0' + idx } else { b'a' + idx - 10 }) as char
        })
        .collect()
}

struct FlockGuard {
    // held for drop — flock released when fd closes
    #[allow(dead_code)]
    file: File,
}

impl Drop for FlockGuard {
    fn drop(&mut self) {
        // flock released implicitly when self.file fd closes
    }
}

fn acquire_flock(path: &str) -> Result<FlockGuard> {
    if let Some(parent) = Path::new(path).parent() {
        let _ = fs::create_dir_all(parent);
    }
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .open(path)
        .with_context(|| format!("cannot open lock file {path}"))?;

    // LOCK_EX | LOCK_NB
    let fd = std::os::unix::io::AsRawFd::as_raw_fd(&file);
    let ret = unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) };
    if ret != 0 {
        anyhow::bail!("zm log dump already running (flock on {path} failed)");
    }

    Ok(FlockGuard { file })
}

fn set_perms(path: &Path, mode: u32) -> Result<()> {
    fs::set_permissions(path, fs::Permissions::from_mode(mode))
        .with_context(|| format!("cannot set permissions on {}", path.display()))
}

fn write_file(path: &Path, data: &[u8]) -> Result<u64> {
    fs::write(path, data)
        .with_context(|| format!("cannot write {}", path.display()))?;
    set_perms(path, 0o644)?;
    Ok(data.len() as u64)
}

fn copy_log_files(
    log_dir: &Path,
    dump_dir: &Path,
    manifest: &mut Vec<serde_json::Value>,
    total: &mut u64,
) -> Result<()> {
    let names = [
        "zeromount.log",
        "zeromount.log.1",
        "zeromount.log.2",
        "zeromount.log.3",
        "zeromount.log.4",
    ];

    for name in &names {
        let src = log_dir.join(name);
        if !src.exists() {
            continue;
        }
        let size = src.metadata().map(|m| m.len()).unwrap_or(0);
        let dst_name = format!("{}.log", random_name(6));
        let dst = dump_dir.join(&dst_name);
        fs::copy(&src, &dst).with_context(|| format!("copy {} failed", src.display()))?;
        set_perms(&dst, 0o644)?;
        *total += size;
        manifest.push(serde_json::json!({
            "file": dst_name,
            "description": format!("rotating log: {name}"),
            "bytes": size,
        }));
    }

    Ok(())
}

fn collect_dmesg(filter: &str, dest: &Path) -> Result<u64> {
    let output = Command::new("dmesg")
        .output()
        .context("dmesg failed")?;

    let filtered: Vec<&[u8]> = output.stdout
        .split(|&b| b == b'\n')
        .filter(|line| {
            let lower = line.to_ascii_lowercase();
            lower.windows(filter.len()).any(|w| w == filter.as_bytes())
        })
        .collect();

    let mut buf: Vec<u8> = Vec::new();
    for line in &filtered {
        buf.extend_from_slice(line);
        buf.push(b'\n');
        if buf.len() as u64 >= DMESG_SIZE_LIMIT {
            break;
        }
    }
    buf.truncate(DMESG_SIZE_LIMIT as usize);

    write_file(dest, &buf)
}

fn dump_config(config: &crate::core::config::ZeroMountConfig, dest: &Path) -> Result<u64> {
    let toml = toml::to_string_pretty(config).context("config serialization failed")?;
    write_file(dest, toml.as_bytes())
}

fn dump_sysfs_level(dest: &Path) -> Result<u64> {
    let content = match super::sysfs::read_kernel_debug_level() {
        Ok(level) => format!("kernel_debug_level={level}\n"),
        Err(e) => format!("kernel_debug_level=unavailable ({e})\n"),
    };
    write_file(dest, content.as_bytes())
}

fn dump_susfs_probe(dest: &Path) -> Result<u64> {
    let content = match crate::susfs::SusfsClient::probe() {
        Ok(client) => {
            let available = client.is_available();
            let version = client.version().unwrap_or("unknown").to_string();
            let features = client.features();
            format!(
                "available={available}\nversion={version}\nkstat={}\npath={}\nmaps={}\nkstat_redirect={}\n",
                features.kstat,
                features.path,
                features.maps,
                features.kstat_redirect,
            )
        }
        Err(e) => format!("probe_error={e}\n"),
    };
    write_file(dest, content.as_bytes())
}
