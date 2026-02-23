#!/system/bin/sh
MODDIR="${0%/*}"

. "$MODDIR/common.sh"
[ -z "$ABI" ] && exit 0
[ -x "$BIN" ] || exit 0

rm -f /data/adb/zeromount/.bootcount

EXTERNAL_SUSFS=$(cat /data/adb/zeromount/flags/external_susfs 2>/dev/null)

# kernel_umount via ksud — only when no external module handles it
if [ "$EXTERNAL_SUSFS" = "none" ] || [ -z "$EXTERNAL_SUSFS" ]; then
    if [ "$("$BIN" config get brene.kernel_umount 2>/dev/null)" = "true" ]; then
        KSUD=""
        [ -x /data/adb/ksu/bin/ksud ] && KSUD=/data/adb/ksu/bin/ksud
        [ -z "$KSUD" ] && [ -x /data/adb/ap/bin/ksud ] && KSUD=/data/adb/ap/bin/ksud
        if [ -n "$KSUD" ]; then
            "$KSUD" feature set kernel_umount 1 2>/dev/null && \
                "$KSUD" feature save 2>/dev/null
            echo "$LOG: kernel_umount enabled via ksud" > /dev/kmsg 2>/dev/null
        fi
    fi
else
    echo "$LOG: kernel_umount deferred to external module ($EXTERNAL_SUSFS)" > /dev/kmsg 2>/dev/null
fi

# Emoji needs pm (package manager), only available post-boot
"$BIN" emoji apply-apps 2>/dev/null || true

# vold-app-data needs FUSE sdcard — wait like official susfs4ksu
if [ "$("$BIN" config get brene.emulate_vold_app_data 2>/dev/null)" = "true" ]; then
    until [ -d "/sdcard/Android/data" ]; do sleep 1; done
    "$BIN" vold-app-data 2>/dev/null || true
fi
