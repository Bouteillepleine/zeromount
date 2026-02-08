# ZeroMount v2 — Architecture Decisions

> **Project:** ZeroMount Metamodule (Rust rewrite)
> **Date Started:** 2026-02-08
> **Architecture:** Rust binary + VFS kernel driver + OverlayFS/Magic Mount fallback
> **Targets:** KernelSU + APatch
> **Previous decisions:** Archived at `docs/DECISIONS-v1.md` (33 items from v1 shell-based architecture)

---

## Architecture Summary

ZeroMount v2 is a Rust-based metamodule that manages module mounting via a **strategy cascade**:

1. **VFS redirection** (primary) — kernel-level path interception, invisible to `/proc/mounts`
2. **OverlayFS** (fallback) — standard overlay mounting with new mount API + legacy fallback
3. **Magic Mount** (last resort) — bind mounts for maximum kernel compatibility

The binary replaces ~2500 lines of shell scripts with a typestate-enforced pipeline. SUSFS becomes an external dependency with build-time patches instead of a maintained fork.

---

## Decision Status Legend

- **CONFIRMED** — Explicitly agreed with user
- **DECIDED** — Recommended with rationale, no objection raised
- **PENDING** — Needs user input before implementation

---

## R: Rust Binary Architecture

### R01: Build from scratch, not fork hybrid mount
**Status:** CONFIRMED

Build a new Rust binary inspired by hybrid mount and mountify patterns but owned entirely by this project. No GPL-3.0 licensing obligation, architecture fits ZeroMount's unique VFS+mount hybrid approach that no existing project implements.

### R02: Typestate pipeline pattern
**Status:** DECIDED

Use `MountController<S>` with consuming state transitions: `Init → Detected → Planned → Mounted → Finalized`. Each transition consumes `self` and returns the next state, making out-of-order operations a compile error. Eliminates the class of bugs seen in the shell scripts (BUG-M3: enable-before-SUSFS race).

### R03: Module scanning with rayon parallelism
**Status:** DECIDED

Parallel module discovery via `rayon::par_iter`. Filter `disable`, `remove`, `skip_mount` sentinels. Blacklist self-name and reserved dirs (`lost+found`, `.git`). Sort reverse-alphabetical for deterministic ordering. Single-pass conflict detection over merged file map replaces the O(n*m) awk approach from `metamount.sh:107-151`.

### R04: TOML configuration with 3-layer resolution
**Status:** DECIDED

Config at `/data/adb/zeromount/config.toml`. Resolution: compiled defaults → config file → CLI overrides. TOML chosen over JSON (needs comments for user-facing config) and shell sourcing (fragile, injection risk). Supports per-module rules and partition overrides.

### R05: CLI subcommand design for WebUI communication
**Status:** DECIDED

Single binary replaces both `zm` (C ioctl wrapper) and all shell orchestration. Key subcommands: `mount` (pipeline), `status` (JSON output), `module list/scan`, `config get/set`, `vfs add/del/clear/enable/disable/refresh/list/query-status`, `uid block/unblock`, `diag`, `version`. The `status` subcommand eliminates the need for `monitor.sh` to regenerate `.status_cache.json` every 5 seconds.

### R06: Logging via tracing crate
**Status:** DECIDED

Dual subscribers: `/dev/kmsg` for kernel log integration + file rotation at `/data/adb/zeromount/logs/`. Replaces 393-line `logging.sh`. Structured spans (module, partition context) replace `log_section` pattern. Verbose mode via `.verbose` file or `--verbose` flag.

### R07: Error handling with anyhow + thiserror
**Status:** DECIDED

`anyhow` for application glue, `thiserror` for domain-specific mount/ioctl/scan errors. Graceful degradation: no single module failure aborts the pipeline. Each mount attempt is error-handled independently — log, record in status, continue to next module.

### R08: Process camouflage
**Status:** DECIDED

Full camouflage fixing BUG-L3. Set both `/proc/self/comm` via `prctl(PR_SET_NAME)` AND `/proc/self/cmdline` via argv[0] overwrite. Current `monitor.sh` only sets `comm`, leaving `cmdline` as `sh monitor.sh`. Rust binary controls argv directly.

### R09: Cross-compilation via cargo-ndk
**Status:** DECIDED

Targets: `aarch64-linux-android` (primary) + `armv7-neon-linux-androideabi` (legacy). NDK API level 21. Optional `build-std` for size optimization. `RUSTFLAGS="-C default-linker-libraries"` for Android compatibility.

### R10: Binary size optimization
**Status:** DECIDED

Release profile: `lto = true`, `strip = true`, `panic = "abort"`, `opt-level = "z"`, `codegen-units = 1`. Target under 1MB. If exceeded: `build-std` with `panic_immediate_abort`, minimize dependency tree.

---

## ME: Mount Engine

### ME01: Storage backend cascade — EROFS → tmpfs → ext4
**Status:** CONFIRMED (ext4 fallback confirmed by user)

Three-tier storage for overlay lower layers. EROFS preferred (compressed read-only, matches Android's native partition format, detection-resistant). tmpfs fallback if kernel supports xattr (`CONFIG_TMPFS_XATTR=y`). ext4 loopback image as last resort for maximum compatibility. Backend selected by runtime capability probing at boot.

### ME02: OverlayFS mounting — new mount API + legacy fallback
**Status:** DECIDED

New `fsopen`/`fsmount`/`move_mount` API (Linux 5.2+) preferred for structured error reporting. Legacy `mount()` syscall as fallback. All KSU-supported kernels are 5.x+, so new API should be available. Source name set per ME09.

### ME03: Magic mount fallback algorithm
**Status:** DECIDED

Bind-mount-based fallback for kernels without OverlayFS. Limitations: no whiteouts, no opaque directories, every file creates a visible `/proc/mounts` entry. Limitations logged and surfaced in status JSON. Ensures ZeroMount functions (with reduced capability) even on minimal kernels.

### ME04: Per-module overlay-to-magic fallback
**Status:** DECIDED

When overlayfs fails for a specific module, fall back to magic mount for *that module only* rather than failing globally. Maximizes successfully-mounted modules. Status output records which strategy was actually used per module.

### ME05: BFS planner — never mount at partition roots
**Status:** DECIDED

Breadth-first planner determines minimum overlay mount points. Hard constraint: never overlay-mount at `/system`, `/vendor`, etc. directly — always mount one level deeper (`/system/bin`, `/vendor/lib`). Matches mountify's `controlled_depth()` and hybrid mount's sensitive partition splitting.

### ME06: SAR child overlay handling
**Status:** DECIDED

Detect System-as-Root symlink situation: `/product` may be symlink to `/system/product` (legacy) or a separate mount point (modern SAR). Resolve before mounting to avoid overlaying symlinks. Matches mountify.sh line 150 pattern.

### ME07: Atomic rename for module content sync
**Status:** DECIDED

Stage module files to `.tmp_<id>`, atomic rename to final path. Backup old version to `.backup_<id>` during swap, restore on failure, delete backup on success. Prevents any process from seeing half-prepared lower layers. Matches hybrid mount's `sync.rs` pattern.

### ME08: Full whiteout and opaque directory support
**Status:** DECIDED

Support all three whiteout formats: character device (`mknod c 0 0`), xattr (`trusted.overlay.whiteout=y`), AUFS (`.wh.*` files). Plus opaque directories (`trusted.overlay.opaque=y`). All three exist in the wild across KSU/Magisk module ecosystems. Matches `metamount.sh:171-207` detection logic.

### ME09: Source="KSU" on all mounts
**Status:** DECIDED

Hardcode mount source name to `"KSU"` for all overlay/tmpfs mounts. Required for KernelSU's `try_umount` to recognize and reverse these mounts per-app. Without this, per-app mount hiding breaks entirely. Configurable source name deferred to v2 (APatch may need different value).

### ME10: KSU try_umount integration
**Status:** DECIDED

Register all mount points with KSU's try_umount system after overlay creation. If SUSFS available, use `add_try_umount`. Otherwise use KSU native `kernel_umount` feature. VFS mode uses its own UID blocklist (separate mechanism, no mounts to unmount).

### ME11: Random mount paths
**Status:** DECIDED

All staging areas use randomized 12-char alphanumeric paths under `/mnt/` (or `/mnt/vendor/` fallback). Fixed paths are fingerprintable. Random paths require brute-force enumeration by detection tools.

### ME12: NukeExt4Sysfs — destroy backing evidence
**Status:** DECIDED

When using ext4 loop images, delete backing file after mount (kernel keeps inode alive via open reference). Hide loop device from sysfs via SUSFS if available. EROFS images also nuked after mount.

---

## VFS: VFS Integration (ZeroMount-specific)

### VFS01: VFS as primary strategy when kernel patches detected
**Status:** CONFIRMED

When `/dev/zeromount` exists and responds to `GET_VERSION` ioctl, all modules use VFS redirection. No mounts created. Invisible to `/proc/mounts`. OverlayFS/magic mount only used when kernel patches are absent.

### VFS02: No mixed VFS/overlay mode
**Status:** DECIDED

Within a single boot, all modules use the same strategy. No per-module VFS vs overlay split. VFS and overlay operate at different layers — mixing them for the same partition creates unpredictable resolution order. Scenario detection runs once at boot, selects one strategy.

### VFS03: VFS ioctl interface rewritten in Rust
**Status:** DECIDED

All 10 ioctl interactions from `zm.c` (304 lines freestanding C) rewritten using `libc` crate. Proper `_IOW`/`_IOR` macro derivation fixes ARM32 compatibility (VFS05). Error messages include errno. `REFRESH` command implemented (fixes BUG-M1).

### VFS04: Ghost directory bug mitigation
**Status:** DECIDED

Kernel bug (BUG-H1: `dirs_ht` not cleaned) requires kernel patch fix. Rust binary mitigates by using `CLEAR_ALL` + full re-injection instead of individual `DEL_RULE` for hot-reload. Kernel patch should be separately updated to clean `dirs_ht` in both `del_rule` and `clear_all`.

### VFS05: ARM32 ioctl compatibility via compile-time derivation
**Status:** DECIDED

Use `_IOW`/`_IOR` macros that compute struct sizes at compile time. ARM64 build produces `0x40185A01` (24-byte struct), ARM32 build produces `0x400C5A01` (12-byte struct) automatically. Fixes BUG-H2 by design — no runtime detection needed.

### VFS06: Engine status query ioctl
**Status:** DECIDED

New `GET_STATUS` ioctl (proposed `0x80045A0B`) returns `zeromount_enabled` atomic value. Fixes BUG-M4 (`isEngineActive()` checking device existence instead of engine state). Backward-compatible fallback returns "unknown" if kernel lacks the new ioctl.

### VFS07: dcache refresh implementation
**Status:** DECIDED

Implement `IOCTL_REFRESH` (`0x5A0A`) handler in CLI. Called automatically at end of VFS pipeline after `enable`. Fixes BUG-M1 (kernel defines ioctl, old binary never exposed it). Pipeline ordering: inject rules → apply SUSFS → enable → refresh. This also fixes BUG-M3 (SUSFS applied before enable).

---

## S: SUSFS Integration

### S01: Build-time patching, not fork maintenance
**Status:** CONFIRMED

Clone upstream SUSFS at pinned commit during CI. Apply ZeroMount-specific patches via `git apply`. Get upstream updates for free. No rebase burden. Same methodology used for KernelSU-Next SUSFS integration.

### S02: Custom SUSFS kernel patches in ZeroMount patch chain
**Status:** DECIDED

The `zeromount_is_uid_blocked` export and `#ifdef CONFIG_ZEROMOUNT` guards at SUSFS check points move into a separate patch file (`zeromount-susfs-coupling.patch`) applied after both SUSFS and ZeroMount core patches. SUSFS upstream stays untouched.

### S03: SUSFS binary interactions moved to Rust
**Status:** DECIDED

The 978-line `susfs_integration.sh` is absorbed into the Rust binary. Direct execution of `ksu_susfs` binary replaces per-path shell-out. Type-safe kstat struct handling replaces `cut -d'|'` parsing. In-memory metadata replaces MD5-keyed file cache. Four capabilities retained: kstat spoofing, path hiding, maps hiding, font redirect.

### S04: SUSFS API capabilities used
**Status:** DECIDED

- **kstat spoofing** (`add_sus_kstat_statically/redirect`) — critical for font/emoji modules
- **Path hiding** (`add_sus_path/loop`) — hide module sources from detection apps
- **Maps hiding** (`add_sus_map`) — hide injected libraries from `/proc/pid/maps`
- **Font redirect** (`add_open_redirect` + kstat) — specialized font module handling
- **Mount hiding** — NOT used (see S05)

### S05: Remove `susfs_apply_mount_hiding()` entirely
**Status:** CONFIRMED

Root cause of LSPosed instability (ARCH-3). The function scans `/proc/mounts` for overlay/tmpfs and hides them via SUSFS. ZeroMount is mountless — it never creates mounts. The scan catches LSPosed, stock Android overlays, and other modules' mounts. Removing eliminates the bug completely.

### S06: Keep `zeromount_is_uid_blocked` kernel export
**Status:** DECIDED

SUSFS consumes this at 3 check points for per-UID visibility decisions. Without it, a UID excluded from ZeroMount would still have SUSFS protections applied, creating inconsistency. The export moves to the ZeroMount patch chain per S02.

---

## DET: 4-Scenario Detection System

### DET01: Scenario definitions
**Status:** DECIDED

| Scenario | Kernel Driver | SUSFS Binary | Strategy |
|----------|--------------|-------------|----------|
| **FULL** | `/dev/zeromount` present | Full capabilities | VFS + full SUSFS protections |
| **SUSFS_FRONTEND** | `/dev/zeromount` present | Partial capabilities | VFS + available SUSFS subset |
| **KERNEL_ONLY** | `/dev/zeromount` present | Not found | VFS only, no metadata spoofing |
| **NONE** | Not present | N/A | OverlayFS/Magic Mount fallback |

### DET02: Kernel capability probing at boot
**Status:** DECIDED

Probe order: (1) `/dev/zeromount` existence, (2) `GET_VERSION` ioctl for driver version, (3) `/proc/filesystems` for zeromount entry, (4) `/proc/config.gz` for `CONFIG_ZEROMOUNT=y`. Steps 1+2 always, 3-4 only on first boot or version mismatch.

### DET03: SUSFS binary availability detection
**Status:** DECIDED

Search order: `/data/adb/ksu/bin/ksu_susfs` → `/data/adb/ksu/bin/susfs` → `$PATH`. Probe capabilities by parsing help output for subcommand names. All 7 capabilities present = FULL, some = SUSFS_FRONTEND, none/missing = KERNEL_ONLY.

### DET04: inotify-based event watching
**Status:** CONFIRMED

Replace 5-second polling loop in `monitor.sh` with `inotify` watches on `/data/adb/modules/`. Instant detection with zero polling overhead. Watch `IN_CREATE | IN_DELETE | IN_MOVED_TO | IN_MOVED_FROM | IN_MODIFY`. Fallback to 10-second polling if inotify_init1 fails.

### DET05: Strategy selection logic
**Status:** DECIDED

Centralized `select_strategy()` maps scenario to capability flags. The module injection loop never checks capabilities directly — it calls through the strategy struct which no-ops unavailable features. `mount_hide` is always `false` per S05.

### DET06: Graceful degradation on capability loss
**Status:** DECIDED

Capabilities probed once at boot and cached. If SUSFS binary disappears mid-session, VFS continues (kernel driver is independent). Existing SUSFS registrations persist in kernel until reboot. Status JSON updated with degradation flag for WebUI display.

### DET07: Runtime status reporting
**Status:** DECIDED

Status JSON at `/data/adb/zeromount/.status_cache.json` includes: scenario, capability flags, driver version, rule/exclusion/hidden-path counts, engine active state, SUSFS version, timestamp. WebUI reads via `ksu.exec("zeromount status")` on demand.

---

## KSU: KernelSU/APatch Platform Integration

### KSU01: Target KernelSU + APatch
**Status:** CONFIRMED

Metamodule mode on both. Not Magisk (no metamodule concept). APatch adopted KernelSU's metamodule system.

### KSU02: Root manager detection
**Status:** DECIDED

Check `$KSU` and `$APATCH` environment variables. Filesystem fallback: `/data/adb/ksu/` or `/data/adb/ap/`. Rust binary abstracts behind a `RootManager` trait for path differences (BusyBox, SUSFS binary, config dirs).

### KSU03: Config storage abstraction
**Status:** DECIDED

KernelSU has `ksud module config` (32 keys, 1MB values). APatch does not. The Rust binary uses file-based config (TOML) as the universal approach, avoiding platform-specific config APIs. `ksud module config` used only for `override.description` (KSU05) and `manage.kernel_umount` when on KSU.

### KSU04: No `manage.kernel_umount` declaration
**Status:** DECIDED

ZeroMount uses VFS redirection, not mounts. Declaring `kernel_umount` would cause ksud to try unmounting nonexistent mount points. In overlay fallback mode (DET01 NONE scenario), ZeroMount manages its own try_umount registration (ME10) rather than delegating to KSU's automatic system.

### KSU05: Dynamic description via `override.description`
**Status:** DECIDED

Update `module.prop` description after pipeline completion: `"GHOST | N Module(s) | mod1, mod2, mod3"` when active, `"Idle"` when no modules, `"ERROR: [reason]"` on failure. Cross-platform (both KSU and APatch display module.prop description).

### KSU06: Thin `metamount.sh` launcher
**Status:** DECIDED

Under 30 lines. Detects architecture, selects correct binary, executes `zeromount mount`, handles bootloop counter, calls `ksud kernel notify-module-mounted` on success. All logic from the current 427-line `metamount.sh` moves into the Rust binary's `mount` subcommand.

### KSU07: `metainstall.sh` — partition normalization at install
**Status:** DECIDED

Detect which partitions exist on the device at install time. Write `partitions.conf` for the Rust binary. Eliminates BUG-M2 (4 scripts with different partition lists) by detecting once rather than guessing at boot.

### KSU08: `metauninstall.sh` — cleanup
**Status:** DECIDED

Clear VFS rules, disable engine, remove `/data/adb/zeromount/` data directory, clean SUSFS entries tagged `[ZeroMount]`. Current 17-line script is appropriate with SUSFS cleanup addition.

### KSU09: `notify-module-mounted` after full pipeline
**Status:** DECIDED

Call AFTER: rules injected, engine enabled, SUSFS applied, kstat pass complete, module description updated. Fixes BUG-M3 race — no window where detection apps observe unspoofed metadata.

### KSU10: `post-fs-data.sh` for detection, `metamount.sh` for mounting
**Status:** DECIDED

Split: `post-fs-data.sh` runs the Rust binary's `detect` subcommand (kernel probe, SUSFS probe, writes detection result JSON). `metamount.sh` reads detection result and runs the `mount` pipeline. Separates lightweight probing (safe at post-fs-data time) from heavy I/O (module iteration, file copying).

---

## W: WebUI

### W01: SolidJS + Material Web Components
**Status:** DECIDED

Continue existing stack. Add `ScenarioIndicator` component showing active detection scenario with colored badge (green=FULL, yellow=SUSFS_FRONTEND, orange=KERNEL_ONLY).

### W02: JSON stdout for binary communication
**Status:** DECIDED

WebUI calls `ksu.exec("zeromount status --json")` and parses stdout. No hex encoding needed — JSON is safe for ksu.exec transport. Simpler than hybrid mount's hex-encoded payload pattern.

### W03: Settings tab with capability-aware toggles
**Status:** DECIDED

Hierarchical toggles: SUSFS Integration (parent) → kstat/path/maps/font sub-toggles. Sub-toggles disabled when parent capability unavailable. Replace dead toggles (`autoStartOnBoot`, `animationsEnabled`) with real controls. Reboot notice for verbose logging (per BUG-L5).

### W04: Scenario display in StatusTab
**Status:** DECIDED

Read `scenario` from status JSON. Display colored chip: "Full Protection" (green), "Partial Protection" (yellow + missing capabilities list), "VFS Only" (orange + warning), "Mount Fallback" (red).

### W05: Fix build output path
**Status:** CONFIRMED

Fix `vite.config.ts` `outDir` from `webroot-beta` to `webroot`. BUG-M7 carry-over.

### W06: Remove all dead code
**Status:** DECIDED

Clean sweep of 15 dead code items from CONTEXT.md Section 9.1: `hitsToday`, `.header__sun`, unused theme imports, dead toggles, unused store methods, nonexistent `installed_apps.json` fetch, always-zero `VfsRule.hits`, always-true `VfsRule.active`, etc.

### W07: Fix store pattern consistency
**Status:** DECIDED

SettingsTab directly imports `api` instead of using store (ARCH-7). Move to store actions for consistency with other tabs.

---

## B: Build System

### B01: SUSFS clone + patch in CI
**Status:** DECIDED

CI clones upstream SUSFS at pinned commit (stored in `susfs-version.txt`), applies ZeroMount coupling patch via `git apply`, fails CI if patch rejects. Ensures reproducible builds and immediate detection of upstream incompatibility.

### B02: Rust cross-compilation
**Status:** DECIDED

`cargo-ndk` with NDK API 21. Targets: `aarch64-linux-android` + `armv7-linux-androideabi`. Static `std` with aggressive LTO + strip. Optional `build-std` if size exceeds target.

### B03: WebUI build integration
**Status:** DECIDED

Separate CI step: `cd webui/ && npm ci && npm run build`. Output to `module/webroot/`. Parallel with Rust build.

### B04: Module ZIP packaging
**Status:** DECIDED

Final ZIP contains: `module.prop`, `customize.sh`, `metainstall.sh`, `metamount.sh`, `metauninstall.sh`, `service.sh`, `post-fs-data.sh`, `zm-arm64`, `zm-arm`, `bin/` (aapt), `webroot/`. Eliminates 5 shell scripts (~2200 lines): `logging.sh`, `susfs_integration.sh`, `monitor.sh`, `sync.sh`, `zm-diag.sh` — all absorbed into Rust binary.

### B05: Module-only CI pipeline
**Status:** CONFIRMED

Builds Rust binary + WebUI + packages ZIP. Kernel patches tested separately by users building their own kernel. Standard approach for metamodules. Full kernel integration CI deferred.

---

## CO: Carry-Over Fixes from v1

### CO01: Ghost directory entries — kernel patch fix
**Status:** DECIDED

BUG-H1. Modify kernel `del_rule` to clean `dirs_ht` child entries. Modify `clear_all` to iterate and free `dirs_ht`. Rust binary mitigates via `CLEAR_ALL` + re-inject pattern for hot-reload.

### CO02: Centralize partition list
**Status:** DECIDED

BUG-M2 + ARCH-1. Single `TARGET_PARTITIONS` Rust constant — union of all 4 current lists (23 unique partitions). Optional install-time detection writes filtered `partitions.conf` (per KSU07).

### CO03: Enable-before-SUSFS race fix
**Status:** DECIDED

BUG-M3. Rust pipeline enforces: inject rules → apply SUSFS → enable → refresh. Type system prevents calling `enable()` before SUSFS completion. See VFS07 for implementation.

### CO04: Version string consistency
**Status:** DECIDED

BUG-L1. Single source of truth: `module.prop:version`. Rust binary reads it at startup, exposes via `zeromount version` and status JSON. Remove hardcoded versions from `constants.ts` and `package.json`.

---

## Decision Count

| Category | Count | CONFIRMED | DECIDED |
|----------|-------|-----------|---------|
| Rust Binary (R) | 10 | 1 | 9 |
| Mount Engine (ME) | 12 | 1 | 11 |
| VFS Integration (VFS) | 7 | 1 | 6 |
| SUSFS Integration (S) | 6 | 3 | 3 |
| Detection System (DET) | 7 | 1 | 6 |
| Platform Integration (KSU) | 10 | 1 | 9 |
| WebUI (W) | 7 | 1 | 6 |
| Build System (B) | 5 | 1 | 4 |
| Carry-Over Fixes (CO) | 4 | 0 | 4 |
| **Total** | **68** | **10** | **58** |

---

*Last updated: 2026-02-08 — Session 2*
