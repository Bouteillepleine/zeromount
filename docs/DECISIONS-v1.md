# ZeroMount Metamodule — Refactoring Decisions

> **Project:** ZeroMount Slim (full-stack refinement)
> **Date Started:** 2026-02-04
> **Last Updated:** 2026-02-08
> **Goal:** Fix instability (LSPosed, mount hijacking), fix confirmed bugs, clean up dead code — while preserving ZeroMount's unique capabilities
> **Scope:** Kernel patches, zm binary, userspace scripts, WebUI

---

## Architecture Context

| Aspect | ZeroMount | Hybrid Mount (Reference) |
|--------|-----------|--------------------------|
| Mechanism | Kernel VFS redirection (`/dev/zeromount`) | Userspace overlayfs/bind mounts |
| Detectability | Invisible | Visible in `/proc/mounts` |
| Font/Emoji Support | Yes (kstat spoof) | No (inode mismatch) |
| Debloat | Yes (VFS-level hiding) | No |
| App Systemizer | Yes | No |
| Statfs/Xattr spoofing | Yes | No |

**Key insight:** ZeroMount is mountless. SUSFS integration is needed for kstat spoofing and path hiding, NOT mount hiding.

---

## Decision Log

### KERNEL LAYER

---

#### DECISION-K01: Ghost directory entries (`dirs_ht` cleanup)

**Bug:** BUG-H1. `del_rule` removes from `rules_ht`/`ino_ht` but NOT `dirs_ht`. `clear_all` clears `rules_ht`/`uid_ht` but NOT `dirs_ht`. Deleted files persist in readdir output.

**Recommendation:** 🔴 FIX
- `del_rule`: remove child entry from parent's dir_node; if dir_node has no children, remove it
- `clear_all`: add `dirs_ht` cleanup alongside existing `rules_ht`/`uid_ht` cleanup

**User Input:** *(pending)*

**Final Decision:** ⏳ PENDING

---

#### DECISION-K02: ARM32 native ioctl mismatch

**Bug:** BUG-H2. `zm.c` hardcodes arm64 struct size in ioctl numbers. ADD_RULE/DEL_RULE broken on native arm32 kernels.

**Recommendation:** 🟡 ACCEPT — pure arm32 Android kernels are extremely rare. Document as known limitation. If fixing: use `_IOW`/`_IOR` macros that compute size at compile time instead of hardcoding.

**User Input:** *(pending)*

**Final Decision:** ⏳ PENDING

---

#### DECISION-K03: Add "is enabled" query

**Bug:** BUG-M4. No ioctl or sysfs attribute exposes `zeromount_enabled` state. WebUI `isEngineActive()` checks device existence instead.

**Recommendation:** 🟡 REVIEW — could add a simple `GET_STATUS` ioctl or expose via existing sysfs. Low priority unless WebUI needs accurate real-time state.

**User Input:** *(pending)*

**Final Decision:** ⏳ PENDING

---

#### DECISION-K04: Kernel 5.10 patch maintenance

**Issue:** Missing statfs/xattr/stat spoofing vs main patch. Possibly unmaintained.

**Recommendation:** 🟡 REVIEW — decide whether to maintain, update, or deprecate.

**User Input:** *(pending)*

**Final Decision:** ⏳ PENDING

---

#### DECISION-K05: Per-function SUSFS bypass guards

**Issue:** `fix-zeromount-susfs-bypass.sh` adds `susfs_is_current_proc_umounted()` to 10 functions. Central `zeromount_should_skip()` already includes this check.

**Recommendation:** 🟡 REVIEW — defense-in-depth (covers early-return paths) vs redundancy. May be needed for functions with early returns before `zeromount_should_skip()`.

**User Input:** *(pending)*

**Final Decision:** ⏳ PENDING

---

### BINARY LAYER (`zm`)

---

#### DECISION-B01: Add `refresh` command

**Bug:** BUG-M1. Kernel defines `ZEROMOUNT_IOC_REFRESH` (0x5A0A). Binary has no handler. `metamount.sh:388` calls `zm refresh` silently failing.

**Recommendation:** 🔴 FIX — add `r` case to dispatch, define `IOCTL_REFRESH = 0x5A0A`, call ioctl with no args. One-line fix.

**User Input:** *(pending)*

**Final Decision:** ⏳ PENDING

---

#### DECISION-B02: Version output format

**Bug:** BUG-L6. `zm ver` outputs bare integer (e.g., "1"). WebUI expects "v3.0.0" format.

**Recommendation:** 🟡 REVIEW — either format in binary or format in WebUI. Binary change is simpler.

**User Input:** *(pending)*

**Final Decision:** ⏳ PENDING

---

### SCRIPT LAYER

---

#### DECISION-S01: Remove `susfs_apply_mount_hiding()`

**File:** `susfs_integration.sh`
**Bug:** ARCH-3. Root cause of LSPosed instability.

**What it does:** Scans `/proc/mounts` for overlay/tmpfs mounts and hides them via SUSFS. ZeroMount doesn't create mounts — catches other systems' mounts.

**Recommendation:** 🔴 REMOVE

**User Input:** *(pending)*

**Final Decision:** ⏳ PENDING

---

#### DECISION-S02: Remove `sus_mount_check` classification

**File:** `susfs_integration.sh` in `susfs_classify_path()`
**Bug:** ARCH-4. Triggers the mount hiding from S01.

**What it does:** Classifies app paths to trigger `susfs_apply_mount_hiding()`.

**Recommendation:** 🔴 REMOVE (the `sus_mount_check` action only — keep other classifications)

**User Input:** *(pending)*

**Final Decision:** ⏳ PENDING

---

#### DECISION-S03: Centralize TARGET_PARTITIONS

**Bug:** BUG-M2. Four scripts define different partition lists (20/10/13/6).

**Recommendation:** 🔴 FIX — extract to a shared sourced file (e.g., `partitions.conf` or a variable in a shared config) that all scripts import.

**User Input:** *(pending)*

**Final Decision:** ⏳ PENDING

---

#### DECISION-S04: Fix enable-before-SUSFS race

**Bug:** BUG-M3. Engine enabled at `metamount.sh:386` before deferred SUSFS at line 399.

**Recommendation:** 🟡 REVIEW — either reorder (apply SUSFS before enable) or accept as known trade-off. Reordering is the safer choice.

**User Input:** *(pending)*

**Final Decision:** ⏳ PENDING

---

#### DECISION-S05: `monitor.sh` — keep, simplify, or remove?

**File:** `monitor.sh` (327 lines)

**Issues:**
- 5-second polling adds overhead
- Force-stops KSU app on app changes
- Camouflage incomplete (comm vs cmdline)
- Hybrid Mount has no background monitor

**Recommendation:** 🟡 REVIEW — user to decide. Options:
1. Keep as-is (functional, feeds WebUI status cache)
2. Simplify (remove force-stop, reduce polling)
3. Remove entirely (WebUI fetches status on-demand)

**User Input:** *(pending)*

**Final Decision:** ⏳ PENDING

---

#### DECISION-S06: `logging.sh` — keep or simplify?

**File:** `logging.sh` (393 lines)

**Recommendation:** 🟡 REVIEW — 393 lines is heavy for logging. But no bugs found, and it's clean utility code. Could simplify if reducing script footprint is a goal.

**User Input:** *(pending)*

**Final Decision:** ⏳ PENDING

---

#### DECISION-S07: Bootloop protection (3-strike)

**File:** `metamount.sh:24-69`

**Recommendation:** ✅ KEEP — good safety mechanism.

**Note:** When bootloop triggers, it doesn't call `zm clear`/`zm disable` — but engine starts DISABLED per `ATOMIC_INIT(0)` so stale rules are harmless within a single boot.

**User Input:** *(pending)*

**Final Decision:** ⏳ PENDING

---

#### DECISION-S08: Whiteout detection

**File:** `metamount.sh:171-207`

**Recommendation:** ✅ KEEP — required for debloat, unique capability.

**User Input:** *(pending)*

**Final Decision:** ⏳ PENDING

---

#### DECISION-S09: Conflict detection

**File:** `metamount.sh:107-151`

**Recommendation:** 🟡 REVIEW — useful but adds boot time. Also duplicated in `zm-diag.sh`. Could be verbose-only or extracted to shared function.

**User Input:** *(pending)*

**Final Decision:** ⏳ PENDING

---

#### DECISION-S10: `susfs_update_config()` — keep or remove?

**File:** `susfs_integration.sh`

**Issue:** ARCH-5. Writes SUSFS config files duplicating runtime commands. Unclear if anything reads them.

**Recommendation:** 🟡 REVIEW — if nothing reads these files, it's dead I/O. User to clarify.

**User Input:** *(pending)*

**Final Decision:** ⏳ PENDING

---

### SUSFS FUNCTIONS (keep/remove)

---

#### DECISION-SF01: `susfs_apply_kstat()`

**Recommendation:** ✅ KEEP — critical for font/emoji modules. Unique capability.

**User Input:** *(pending)*

**Final Decision:** ⏳ PENDING

---

#### DECISION-SF02: `susfs_apply_path()`

**Recommendation:** ✅ KEEP — needed for hiding module source files.

**User Input:** *(pending)*

**Final Decision:** ⏳ PENDING

---

#### DECISION-SF03: `susfs_apply_maps()`

**Recommendation:** ✅ KEEP — needed for hiding injected libraries from `/proc/pid/maps`.

**User Input:** *(pending)*

**Final Decision:** ⏳ PENDING

---

#### DECISION-SF04: `apply_font_redirect()`

**Recommendation:** ✅ KEEP — 115 lines, specialized, but critical for font modules.

**User Input:** *(pending)*

**Final Decision:** ⏳ PENDING

---

### WEBUI LAYER

---

#### DECISION-W01: Fix `isEngineActive()` fallback

**Bug:** BUG-M4. Slow path checks `/dev/zeromount` existence, not engine state.

**Recommendation:** 🟡 REVIEW — depends on K03 (kernel "is enabled" query). If kernel ioctl added, update API. Otherwise, document limitation.

**User Input:** *(pending)*

**Final Decision:** ⏳ PENDING

---

#### DECISION-W02: Fix UID unblock persistence

**Bug:** BUG-M6. `sed` failure swallowed → UID re-blocked on reboot.

**Recommendation:** 🔴 FIX — propagate `sed` error instead of swallowing in `.catch()`.

**User Input:** *(pending)*

**Final Decision:** ⏳ PENDING

---

#### DECISION-W03: Fix build output path

**Bug:** BUG-M7. vite outputs to `webroot-beta`, deployed as `webroot`.

**Recommendation:** 🔴 FIX — update `vite.config.ts` `outDir` to `../module/webroot`.

**User Input:** *(pending)*

**Final Decision:** ⏳ PENDING

---

#### DECISION-W04: Fix version string consistency

**Bug:** BUG-L1. v3.4.0 / 3.0.0 / 0.0.0 across module.prop, constants.ts, package.json.

**Recommendation:** 🔴 FIX — align all to v3.4.0 (or whatever the current version is).

**User Input:** *(pending)*

**Final Decision:** ⏳ PENDING

---

#### DECISION-W05: Fix activity type parser

**Bug:** BUG-L2. Parser only recognizes 6 of 8+ activity types.

**Recommendation:** 🔴 FIX — add missing type cases.

**User Input:** *(pending)*

**Final Decision:** ⏳ PENDING

---

#### DECISION-W06: Remove dead code

**Items:** See CONTEXT.md Section 9.1 (15 dead code items in WebUI).

**Recommendation:** 🟡 REVIEW — clean sweep or selective removal. Options:
1. Remove all dead code now (clean slate)
2. Remove only items that cause confusion (VfsRule naming, hitsToday, installed_apps.json)
3. Leave for later

**User Input:** *(pending)*

**Final Decision:** ⏳ PENDING

---

#### DECISION-W07: Fix VfsRule naming inversion

**Bug:** BUG-L4. `source` = real path, `target` = virtual path — backwards.

**Recommendation:** 🟡 REVIEW — renaming is a cross-cutting change affecting types, API, store, and pages. Low risk but touches many files.

**User Input:** *(pending)*

**Final Decision:** ⏳ PENDING

---

#### DECISION-W08: `autoStartOnBoot` / `animationsEnabled` toggles

**Issue:** Toggles exist in UI but have no backend wiring.

**Recommendation:** 🟡 REVIEW — either implement or remove. Removing is simpler.

**User Input:** *(pending)*

**Final Decision:** ⏳ PENDING

---

### DOCUMENTATION

---

#### DECISION-D01: Update stale ARCHITECTURE.md

**Bug:** BUG-M8. Wrong ioctl count (7 vs 10), wrong binary name (nm vs zm), wrong default state.

**Recommendation:** 🔴 FIX

**User Input:** *(pending)*

**Final Decision:** ⏳ PENDING

---

#### DECISION-D02: Update stale README.md

**Issue:** Documents `nm` binary with `block`/`unblock`. Actual binary is `zm` with `blk`/`unb`, plus `enable`/`disable`.

**Recommendation:** 🔴 FIX

**User Input:** *(pending)*

**Final Decision:** ⏳ PENDING

---

### CLEANUP

---

#### DECISION-C01: Remove accidental `"` file

**Issue:** 1-byte file named `"` in repo root. Shell quoting accident.

**Recommendation:** 🔴 REMOVE

**User Input:** *(pending)*

**Final Decision:** ⏳ PENDING

---

#### DECISION-C02: Archive legacy `nm.c`

**Issue:** 246 lines at `src/legacy/nm.c`, fully superseded by `zm.c`.

**Recommendation:** 🟡 REVIEW — delete or move to archive.

**User Input:** *(pending)*

**Final Decision:** ⏳ PENDING

---

## Summary (after all decisions finalized)

**KEEP:**
- *(to be filled after user review)*

**FIX:**
- *(to be filled after user review)*

**REMOVE:**
- *(to be filled after user review)*

**DEFER:**
- *(to be filled after user review)*

---

## Decision Count

| Category | Count | REMOVE/FIX | KEEP | REVIEW | PENDING |
|----------|-------|------------|------|--------|---------|
| Kernel | 5 | 1 | 0 | 4 | 5 |
| Binary | 2 | 1 | 0 | 1 | 2 |
| Scripts | 10 | 3 | 2 | 5 | 10 |
| SUSFS Functions | 4 | 0 | 4 | 0 | 4 |
| WebUI | 8 | 4 | 0 | 4 | 8 |
| Documentation | 2 | 2 | 0 | 0 | 2 |
| Cleanup | 2 | 1 | 0 | 1 | 2 |
| **Total** | **33** | **12** | **6** | **15** | **33** |

---

## Next Steps

1. User reviews all 33 decisions and provides input
2. Finalize each decision (KEEP/FIX/REMOVE/DEFER)
3. Prioritize implementation order
4. Implement changes in `zeromount-slim/` (scripts), source repos (kernel/binary/WebUI)
5. Test on device
6. If stable, apply to main

---

*Last updated: 2026-02-08*
