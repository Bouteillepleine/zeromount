#!/system/bin/sh
MODDIR="${0%/*}"

. "$MODDIR/common.sh"
[ -z "$ABI" ] && exit 0
[ -x "$BIN" ] || exit 0

rm -f /data/adb/zeromount/.bootcount

# Emoji and vold-app-data need pm (package manager), only available post-boot
"$BIN" emoji apply-apps 2>/dev/null || true
"$BIN" vold-app-data 2>/dev/null || true
