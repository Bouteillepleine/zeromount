# ZeroMount Metamodule Refactoring Decisions

> **Project:** ZeroMount Slim
> **Date Started:** 2026-02-04
> **Goal:** Fix instability (LSPosed, mount hijacking) while preserving ZeroMount's unique capabilities

---

## Architecture Context

| Aspect | ZeroMount | Hybrid Mount (Reference) |
|--------|-----------|--------------------------|
| Mechanism | Kernel VFS redirection (`/dev/zeromount`) | Userspace overlayfs/bind mounts |
| Detectability | Invisible | Visible in `/proc/mounts` |
| Font/Emoji Support | ✅ (needs kstat spoof) | ❌ (inode mismatch) |
| Debloat | ✅ VFS-level hiding | ❌ |
| App Systemizer | ✅ | ❌ |

**Key insight:** ZeroMount is mountless - it doesn't create visible mounts. SUSFS integration is needed for kstat spoofing, NOT mount hiding.

---

## File Inventory

| File | Lines | Status | Notes |
|------|-------|--------|-------|
| `metamount.sh` | 428 | 🟡 REVIEW | Core mounting logic |
| `susfs_integration.sh` | 979 | 🟡 REVIEW | Contains both needed and problematic code |
| `service.sh` | 80 | 🟡 REVIEW | |
| `monitor.sh` | 328 | 🟡 REVIEW | Background polling |
| `logging.sh` | 394 | 🟡 REVIEW | Logging library |
| `sync.sh` | 130 | 🟡 REVIEW | |
| `zm-diag.sh` | 129 | 🟡 REVIEW | Diagnostics |
| `metainstall.sh` | 3 | ✅ KEEP | Minimal, works |
| `metauninstall.sh` | 17 | ✅ KEEP | Minimal, works |
| `customize.sh` | 45 | ✅ KEEP | Installation setup |

---

## Decision Log

### DECISION-001: `susfs_apply_mount_hiding()` function

**File:** `susfs_integration.sh:702-745`

**What it does:**
Scans `/proc/mounts` for overlay/tmpfs mounts and hides them via SUSFS.

**Claude's Recommendation:** 🔴 REMOVE
**Reason:**
- ZeroMount doesn't create mounts, so why hide mounts?
- Catches ALL overlay/tmpfs on device including stock
- Hijacks mounts from other modules (LSPosed)
- Modules with `skip_mount` still get affected

**User Input:** *(pending)*

**Final Decision:** ⏳ PENDING

---

### DECISION-002: `sus_mount_check` in `susfs_classify_path()`

**File:** `susfs_integration.sh:118-153`

**What it does:**
Classifies app paths to trigger `susfs_apply_mount_hiding()` for app directories.

**Claude's Recommendation:** 🔴 REMOVE (the `sus_mount_check` action only)
**Reason:**
- This triggers mount hiding which is the problematic behavior
- Other classifications (sus_path, sus_kstat, sus_maps) should remain

**User Input:** *(pending)*

**Final Decision:** ⏳ PENDING

---

### DECISION-003: `monitor.sh` background process

**File:** `monitor.sh` (328 lines)

**What it does:**
- Polls modules every 5 seconds
- Watches for app installs via inotifywait/logcat
- Force-stops `com.rifsxd.ksunext` on app changes
- Process camouflage as `kworker/u*:zm`

**Claude's Recommendation:** 🟡 REVIEW - possibly simplify or remove
**Reason:**
- Force-stopping KSU may cause issues
- 5s polling adds overhead
- Hybrid Mount has no background monitor

**User Input:** *(pending)*

**Final Decision:** ⏳ PENDING

---

### DECISION-004: `logging.sh` library

**File:** `logging.sh` (394 lines)

**What it does:**
Structured logging with levels, rotation, per-component log files.

**Claude's Recommendation:** 🟡 REVIEW - possibly simplify
**Reason:**
- 394 lines for logging may be over-engineered
- Hybrid Mount uses 5-line log function

**User Input:** *(pending)*

**Final Decision:** ⏳ PENDING

---

### DECISION-005: Bootloop protection (3-strike)

**File:** `metamount.sh:24-69`

**What it does:**
Counts boot attempts, disables module after 3 failures, backup/restore config.

**Claude's Recommendation:** ✅ KEEP
**Reason:**
- Good safety mechanism
- Hybrid Mount has similar (Granary)

**User Input:** *(pending)*

**Final Decision:** ⏳ PENDING

---

### DECISION-006: Whiteout detection (overlay, xattr, AUFS)

**File:** `metamount.sh:171-207`

**What it does:**
Detects overlay whiteouts (char device 0,0), xattr whiteouts, AUFS `.wh.*` files.

**Claude's Recommendation:** ✅ KEEP
**Reason:**
- Required for debloat functionality
- Unique ZeroMount capability

**User Input:** *(pending)*

**Final Decision:** ⏳ PENDING

---

### DECISION-007: Conflict detection

**File:** `metamount.sh:107-151`

**What it does:**
Scans all modules for files touching same path, logs warnings.

**Claude's Recommendation:** 🟡 REVIEW
**Reason:**
- Useful for debugging
- Adds boot time overhead
- Could be optional/verbose-only

**User Input:** *(pending)*

**Final Decision:** ⏳ PENDING

---

### DECISION-008: `susfs_apply_kstat()` function

**File:** `susfs_integration.sh:409-536`

**What it does:**
Spoofs inode, device ID, timestamps for redirected files (critical for fonts/emojis).

**Claude's Recommendation:** ✅ KEEP
**Reason:**
- Required for font/emoji modules to work
- Unique ZeroMount capability
- Hybrid Mount can't do this

**User Input:** *(pending)*

**Final Decision:** ⏳ PENDING

---

### DECISION-009: `susfs_apply_path()` function

**File:** `susfs_integration.sh:220-295`

**What it does:**
Hides paths from visibility via `add_sus_path`.

**Claude's Recommendation:** ✅ KEEP
**Reason:**
- Needed for hiding module source files
- Part of ZeroMount's stealth

**User Input:** *(pending)*

**Final Decision:** ⏳ PENDING

---

### DECISION-010: `susfs_apply_maps()` function

**File:** `susfs_integration.sh:369-405`

**What it does:**
Hides library mappings from `/proc/pid/maps`.

**Claude's Recommendation:** ✅ KEEP
**Reason:**
- Needed for hiding injected libraries
- Detection evasion

**User Input:** *(pending)*

**Final Decision:** ⏳ PENDING

---

### DECISION-011: `apply_font_redirect()` function

**File:** `susfs_integration.sh:539-653`

**What it does:**
Combined open_redirect + kstat_redirect specifically for font files.

**Claude's Recommendation:** ✅ KEEP
**Reason:**
- Critical for font modules
- Unique ZeroMount capability

**User Input:** *(pending)*

**Final Decision:** ⏳ PENDING

---

## Summary Template

After all decisions are made, we'll have:

**KEEP:**
- (list of functions/files to keep as-is)

**MODIFY:**
- (list of functions/files to modify with specific changes)

**REMOVE:**
- (list of functions/files to delete)

---

## Next Steps

1. Review each DECISION item above
2. User provides input/corrections
3. Update Final Decision for each
4. Implement changes in `zeromount-slim/`
5. Test on device
6. If stable, apply to original

---

*Last updated: 2026-02-04*
