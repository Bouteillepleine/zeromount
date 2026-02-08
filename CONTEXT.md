# ZeroMount Metamodule - Complete Context Document

> **Purpose:** Preserve all understanding before context window exhaustion
> **Date:** 2026-02-04
> **Status:** Awaiting SUSFS full context from user

---

## 1. The Problem Statement

**Symptom:** ZeroMount metamodule causes instability with LSPosed and interferes with other modules.

**Root Cause Identified:** The `susfs_apply_mount_hiding()` function in `susfs_integration.sh` scans `/proc/mounts` for ALL overlay/tmpfs mounts and attempts to hide them via SUSFS. Since ZeroMount is a **mountless** VFS redirection system (it doesn't create mounts), this function catches and hides mounts from OTHER systems like LSPosed.

**The Crime:** Trying to hide mounts that ZeroMount didn't create.

---

## 2. ZeroMount Architecture

### 2.1 What ZeroMount IS

ZeroMount is a **kernel-level VFS redirection system**. It intercepts path resolution at the Virtual File System layer and redirects file accesses to different locations - all without creating visible mount points.

```
App opens: /system/lib/libc.so
    ↓
VFS layer (fs/namei.c)
    ↓
zeromount_getname_hook() intercepts
    ↓
Looks up rule: /system/lib/libc.so → /data/adb/modules/xyz/system/lib/libc.so
    ↓
Returns redirected path to VFS
    ↓
App reads from module file (thinks it's /system/lib/libc.so)
```

### 2.2 What ZeroMount is NOT

- NOT an overlayfs-based system (like Magisk/KernelSU magic mount)
- NOT visible in `/proc/mounts`
- NOT detectable by standard mount enumeration
- NOT creating any mount points

### 2.3 Unique Capabilities

| Capability | ZeroMount | Hybrid Mount | Why Different |
|------------|-----------|--------------|---------------|
| Font/Emoji modules | ✅ | ❌ | Needs kstat spoofing for inode match |
| Debloat (remove files) | ✅ | ❌ | VFS returns ENOENT, no whiteout needed |
| App Systemizer | ✅ | ❌ | Injects into /system/app listings |
| Invisible operation | ✅ | ❌ | No /proc/mounts entries |
| Detection evasion | ✅ Higher | ❌ Lower | No mount artifacts |

---

## 3. Kernel Implementation Details

### 3.1 Device Interface

- **Device:** `/dev/zeromount`
- **Type:** misc device (MISC_DYNAMIC_MINOR)
- **Permissions:** 0600 (root only)
- **Access:** Requires root euid to open, CAP_SYS_ADMIN for privileged ioctls

### 3.2 VFS Hooks Installed

| Hook Location | Function | Purpose |
|---------------|----------|---------|
| fs/namei.c `getname_flags()` | `zeromount_getname_hook()` | Main path redirection |
| fs/namei.c `generic_permission()` | Permission check | Allow traversal through /data/adb |
| fs/readdir.c `getdents64()` | `zeromount_inject_dents64()` | Inject virtual directory entries |
| fs/d_path.c `d_path()` | Inode-to-path hook | Return virtual path for redirected files |
| fs/statfs.c `statfs()` | `zeromount_spoof_statfs()` | Spoof filesystem type as EROFS |
| fs/xattr.c `getxattr()` | `zeromount_spoof_xattr()` | Spoof SELinux contexts |

### 3.3 Core Data Structures

```c
struct zeromount_rule {
    struct hlist_node node;      // Hash by virtual_path
    struct hlist_node ino_node;  // Hash by inode
    struct list_head list;       // Linear list for iteration
    char *virtual_path;          // e.g., "/system/lib/libc.so"
    char *real_path;             // e.g., "/data/adb/modules/xyz/system/lib/libc.so"
    unsigned long real_ino;      // Cached inode of real file
    dev_t real_dev;              // Device ID of real file
    bool is_new;                 // Is this a newly injected file?
    u32 flags;                   // ZM_FLAG_ACTIVE, ZM_FLAG_IS_DIR
    struct rcu_head rcu;         // RCU for safe deletion
};

struct zeromount_uid_node {
    uid_t uid;                   // UID to exclude from redirection
    struct hlist_node node;
    struct rcu_head rcu;
};
```

### 3.4 Ioctl Commands

| Command | Code | Purpose |
|---------|------|---------|
| ADD_RULE | 0x40185A01 | Add path redirection rule |
| DEL_RULE | 0x40185A02 | Delete rule |
| CLEAR | 0x5A03 | Clear all rules and UIDs |
| GET_VERSION | 0x80045A04 | Query version (unprivileged) |
| ADD_UID | 0x40045A05 | Exclude UID from redirection |
| DEL_UID | 0x40045A06 | Include UID in redirection |
| GET_LIST | 0x80045A07 | List all rules |
| ENABLE | 0x5A08 | Activate redirection engine |
| DISABLE | 0x5A09 | Deactivate engine |
| REFRESH | 0x5A0A | Flush dcache for all paths |

### 3.5 Recursion Protection

ZeroMount uses per-task recursion guards to prevent infinite loops when `kern_path()` re-enters VFS:

```c
// Uses android_oem_data1 bit 0 (survives CPU migration)
static inline void zm_enter(void) {
    set_bit(ZM_RECURSIVE_BIT, &current->android_oem_data1);
}
static inline void zm_exit(void) {
    clear_bit(ZM_RECURSIVE_BIT, &current->android_oem_data1);
}
static inline bool zm_is_recursive(void) {
    return test_bit(ZM_RECURSIVE_BIT, &current->android_oem_data1);
}
```

---

## 4. Kernel-SUSFS Integration

### 4.1 The Export

ZeroMount exports `zeromount_is_uid_blocked()` via EXPORT_SYMBOL:

```c
bool zeromount_is_uid_blocked(uid_t uid) {
    // Returns true if UID is in exclusion list
    // RCU-protected hash table lookup
}
EXPORT_SYMBOL(zeromount_is_uid_blocked);
```

### 4.2 SUSFS Consumption

SUSFS declares the symbol as `extern` and wraps it:

```c
#ifdef CONFIG_ZEROMOUNT
extern bool zeromount_is_uid_blocked(uid_t uid);
static inline bool susfs_is_uid_zeromount_excluded(uid_t uid) {
    return zeromount_is_uid_blocked(uid);
}
#else
static inline bool susfs_is_uid_zeromount_excluded(uid_t uid) { return false; }
#endif
```

### 4.3 SUSFS Call Points

SUSFS calls `susfs_is_uid_zeromount_excluded()` at 3 check points:

1. **`is_i_uid_in_android_data_not_allowed()`** - Path visibility in /data
2. **`is_i_uid_in_sdcard_not_allowed()`** - Mount visibility in sdcard
3. **`is_i_uid_not_allowed()`** - Generic UID check

**Semantics:** When `zeromount_is_uid_blocked(uid)` returns TRUE:
- ZeroMount: Bypasses VFS redirection (app sees REAL files)
- SUSFS: Bypasses ALL hiding (path, mount, kstat)
- Result: Excluded UID sees the REAL unmodified system

---

## 5. Userspace Implementation

### 5.1 File Inventory

| File | Lines | Purpose |
|------|-------|---------|
| `metamount.sh` | 428 | Main mounting hook - iterates modules, calls `zm add` |
| `susfs_integration.sh` | 979 | SUSFS helper functions (PROBLEM AREA) |
| `service.sh` | 80 | Service stage - hides ZeroMount artifacts, applies exclusions |
| `monitor.sh` | 328 | Background polling, app watcher |
| `logging.sh` | 394 | Structured logging library |
| `sync.sh` | 130 | Module sync functionality |
| `zm-diag.sh` | 129 | Diagnostics tool |
| `metainstall.sh` | 3 | Minimal - calls `install_module` |
| `metauninstall.sh` | 17 | Cleanup - clears rules and data |
| `customize.sh` | 45 | Installation setup |
| **TOTAL** | **2526** | |

### 5.2 Reference Comparison

| Metric | ZeroMount | Hybrid Mount | Mountify |
|--------|-----------|--------------|----------|
| metamount.sh lines | 428 | 30 | 430 |
| SUSFS integration | 979 lines | 0 | 10 lines |
| Background monitor | 328 lines | 0 | 0 |
| Total shell lines | 2526 | 158 | 1062 |

---

## 6. The Problem Functions

### 6.1 `susfs_apply_mount_hiding()` (REMOVE)

**Location:** `susfs_integration.sh:702-745`

**What it does:**
```sh
susfs_apply_mount_hiding() {
    local vpath="$1"
    # Scans /proc/mounts for overlay/tmpfs matching this path
    mount_point=$(awk -v path="$vpath" '
        ($3 == "overlay" || $3 == "tmpfs") && path ~ "^"$2 {
            print $2; exit
        }
    ' /proc/mounts)
    # Then hides whatever it finds via SUSFS
    "$SUSFS_BIN" add_sus_mount "$mount_point"
}
```

**Why it's wrong:**
1. ZeroMount doesn't create mounts, so this catches OTHER systems' mounts
2. LSPosed uses overlay mounts - they get hidden
3. Modules with `skip_mount` still get their mounts hidden
4. Stock Android overlay mounts can be affected

### 6.2 `sus_mount_check` Classification (REMOVE)

**Location:** `susfs_integration.sh:118-153` in `susfs_classify_path()`

**What it does:**
```sh
case "$vpath" in
    /system/app/*|/system/priv-app/*|/product/app/*|...)
        actions="$actions,sus_mount_check"  # Triggers mount hiding
        ;;
esac
```

**Why it's wrong:**
- This classification triggers `susfs_apply_mount_hiding()` for app paths
- Unnecessary for a mountless architecture

---

## 7. Functions to KEEP

### 7.1 `susfs_apply_path()` (KEEP)

Hides paths from visibility via `add_sus_path`. Needed for hiding module source files.

### 7.2 `susfs_apply_kstat()` (KEEP)

Spoofs inode metadata (ino, dev, size, timestamps). **Critical for font/emoji modules** - the inode must match what the system expects.

### 7.3 `susfs_apply_maps()` (KEEP)

Hides library mappings from `/proc/pid/maps`. Needed for hiding injected libraries.

### 7.4 `apply_font_redirect()` (KEEP)

Combined `open_redirect` + `kstat_redirect` for font files. Essential for font module support.

### 7.5 ZeroMount Self-Hiding in `service.sh` (KEEP)

```sh
# service.sh lines 36-42
"$SUSFS_BIN" add_sus_path_loop /dev/zeromount
"$SUSFS_BIN" add_sus_path_loop /sys/kernel/zeromount
```

Hides ZeroMount's own presence from detection apps. **Must preserve.**

---

## 8. Experiment Setup

### 8.1 Directory Structure

```
/home/claudetest/metamodule-experiment/
├── zeromount-original/   # Backup (don't touch)
├── zeromount-slim/       # Working copy (modify here)
├── mountify-reference/   # Reference implementation
├── hybrid-reference/     # Reference implementation
├── DECISIONS.md          # Decision tracking
└── CONTEXT.md            # This document
```

### 8.2 Source Locations

- ZeroMount module: `/home/claudetest/zero-mount/nomount/module/`
- ZeroMount patches: `/home/claudetest/zero-mount/nomount/patches/`
- Hybrid Mount: `/home/claudetest/gki-build/meta-hybrid_mount/`
- Mountify: `/home/claudetest/gki-build/mountify-analysis/`

---

## 9. Decision Summary (Pending User Confirmation)

| ID | Component | Recommendation | Confidence |
|----|-----------|----------------|------------|
| 001 | `susfs_apply_mount_hiding()` | 🔴 REMOVE | HIGH |
| 002 | `sus_mount_check` classification | 🔴 REMOVE | HIGH |
| 003 | `monitor.sh` | 🟡 REVIEW | MEDIUM |
| 004 | `logging.sh` | 🟡 REVIEW | MEDIUM |
| 005 | Bootloop protection | ✅ KEEP | HIGH |
| 006 | Whiteout detection | ✅ KEEP | HIGH |
| 007 | Conflict detection | 🟡 REVIEW | MEDIUM |
| 008 | `susfs_apply_kstat()` | ✅ KEEP | HIGH |
| 009 | `susfs_apply_path()` | ✅ KEEP | HIGH |
| 010 | `susfs_apply_maps()` | ✅ KEEP | HIGH |
| 011 | `apply_font_redirect()` | ✅ KEEP | HIGH |

---

## 10. Open Questions

1. **What other SUSFS hiding in `susfs_integration.sh` is for ZeroMount self-hiding vs mount hijacking?**
   - User indicated there may be more code for hiding ZeroMount itself
   - Need to distinguish legitimate self-hiding from problematic mount scanning

2. **Full SUSFS Context**
   - User will provide kernel-side SUSFS patches
   - User will provide userspace SUSFS documentation
   - This will complete the picture

3. **Are there other functions in `susfs_integration.sh` that scan /proc or interfere with other modules?**

---

## 11. Next Steps

1. ⏳ Receive full SUSFS context from user (kernel + userspace)
2. ⏳ Update this document with SUSFS understanding
3. ⏳ Finalize all DECISIONS with user confirmation
4. ⏳ Implement surgical changes in `zeromount-slim/`
5. ⏳ Deploy validator agents to verify changes
6. ⏳ Test on device

---

*Last updated: 2026-02-04*
*Awaiting: SUSFS full context from user*
