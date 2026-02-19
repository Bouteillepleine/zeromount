#!/bin/bash
# inject-overlay-mount-filter.sh
# Hides overlay mounts from umounted processes in proc_namespace.c
#
# Runs AFTER the SUSFS GKI patch. Targets the same file as
# inject-susfs-mount-display.sh but adds overlay-specific filtering.
#
# Usage: ./inject-overlay-mount-filter.sh <KERNEL_COMMON_DIR>

set -e

KERNEL_DIR="$1"
if [ -z "$KERNEL_DIR" ]; then
    echo "Usage: $0 <KERNEL_COMMON_DIR>"
    exit 1
fi

PROC_NS="$KERNEL_DIR/fs/proc_namespace.c"
if [ ! -f "$PROC_NS" ]; then
    echo "FATAL: $PROC_NS not found"
    exit 1
fi

echo "=== inject-overlay-mount-filter ==="
echo "    Target: $PROC_NS"

# Idempotency
if grep -q '0x794c7630' "$PROC_NS"; then
    echo "[=] Overlay mount filter already present"
    exit 0
fi

# Prerequisite: SUSFS mount display block must exist
if ! grep -q 'susfs_hide_sus_mnts_for_non_su_procs' "$PROC_NS"; then
    echo "FATAL: SUSFS mount display block not found."
    echo "       Apply the SUSFS GKI patch first."
    exit 1
fi

cp "$PROC_NS" "${PROC_NS}.bak"
trap 'mv "${PROC_NS}.bak" "$PROC_NS" 2>/dev/null; exit 1' ERR

# --- 1. Add #include <linux/zeromount.h> if not present ---
if ! grep -q 'linux/zeromount.h' "$PROC_NS"; then
    echo "[+] Adding zeromount.h include"
    sed -i '/#include <linux\/susfs_def.h>/a #include <linux/zeromount.h>' "$PROC_NS"
    if ! grep -q 'linux/zeromount.h' "$PROC_NS"; then
        echo "FATAL: Failed to inject zeromount.h include"
        mv "${PROC_NS}.bak" "$PROC_NS"
        exit 1
    fi
else
    echo "[=] zeromount.h include already present"
fi

# --- 2. Inject overlay filter after existing SUSFS #endif in 3 show_* functions ---
# Anchor: the SUSFS SUS_MOUNT block appears 3 times, each containing
# "r->mnt_id >= DEFAULT_KSU_MNT_ID". We find the #endif that closes
# each block and inject our overlay filter after it.
echo "[+] Injecting overlay filter blocks"
awk '
/r->mnt_id >= DEFAULT_KSU_MNT_ID/ { in_sus_block = 1 }
in_sus_block && /^#endif/ {
    print
    print ""
    print "#ifdef CONFIG_KSU_SUSFS_SUS_MOUNT"
    print "\t/* Hide overlay mounts from umounted processes (toggle) */"
    print "\tif (zeromount_hide_overlays &&"
    print "\t    mnt_path.dentry->d_sb->s_magic == 0x794c7630 &&"
    print "\t    susfs_is_current_proc_umounted())"
    print "\t{"
    print "\t\treturn 0;"
    print "\t}"
    print "#endif"
    in_sus_block = 0
    next
}
{ print }
' "$PROC_NS" > "${PROC_NS}.tmp" && mv "${PROC_NS}.tmp" "$PROC_NS"

# --- 3. Validate: exactly 3 occurrences ---
count=$(grep -c '0x794c7630' "$PROC_NS" || true)
if [ "$count" -ne 3 ]; then
    echo "FATAL: expected 3 overlay filter blocks, found $count"
    mv "${PROC_NS}.bak" "$PROC_NS"
    exit 1
fi

trap - ERR
rm -f "${PROC_NS}.bak"
echo "[+] Overlay mount filter injected into 3 show_* functions"
echo "=== Done ==="
