#!/system/bin/sh
MODDIR="${0%/*}"

case "$(uname -m)" in
    aarch64) ABI=arm64-v8a ;;
    armv7*|armv8l) ABI=armeabi-v7a ;;
    x86_64) ABI=x86_64 ;;
    i686|i386) ABI=x86 ;;
    *) exit 1 ;;
esac

BIN="$MODDIR/bin/${ABI}/zeromount"
[ -x "$BIN" ] || exit 1

# Rust pipeline owns bootcount (increment + reset + threshold).
# Shell fast-fail only: if already past threshold, don't even start.
COUNT=$(cat /data/adb/zeromount/.bootcount 2>/dev/null || echo 0)
[ "$COUNT" -ge 3 ] && exit 1

"$BIN" mount
