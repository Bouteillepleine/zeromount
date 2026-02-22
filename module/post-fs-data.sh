#!/system/bin/sh
MODDIR="${0%/*}"

. "$MODDIR/common.sh"
[ -z "$ABI" ] && exit 0
[ -x "$BIN" ] || exit 0

"$BIN" detect

# ADB Root via adbex injection
ADB_ROOT=$("$BIN" config get adb.adb_root 2>/dev/null)
if [ "$ADB_ROOT" != "true" ]; then
    echo "zeromount: adb_root disabled, skipping adbex injection" > /dev/kmsg 2>/dev/null
    exit 0
fi

ADBEX_PATH=/data/adb/adbex
INJECT="$MODDIR/bin/${ABI}/adbex_inject"

if [ ! -x "$INJECT" ]; then
    echo "zeromount: adbex_inject not found at $INJECT" > /dev/kmsg 2>/dev/null
    exit 0
fi
if [ ! -f "$MODDIR/lib/${ABI}/libadbex_init.so" ]; then
    echo "zeromount: libadbex_init.so not found for ${ABI}" > /dev/kmsg 2>/dev/null
    exit 0
fi
if [ ! -f "$MODDIR/lib/${ABI}/libadbex_adbd.so" ]; then
    echo "zeromount: libadbex_adbd.so not found for ${ABI}" > /dev/kmsg 2>/dev/null
    exit 0
fi

echo "zeromount: staging adbex libraries to $ADBEX_PATH" > /dev/kmsg 2>/dev/null
mkdir -p "$ADBEX_PATH"
cp "$MODDIR/lib/${ABI}/libadbex_init.so" "$ADBEX_PATH/"
cp "$MODDIR/lib/${ABI}/libadbex_adbd.so" "$ADBEX_PATH/"
chcon -R u:object_r:system_file:s0 "$ADBEX_PATH"

# Patch linker config for ADBD APEX namespace
if [ -f /linkerconfig/com.android.adbd/ld.config.txt ]; then
    if ! grep -q "$ADBEX_PATH" /linkerconfig/com.android.adbd/ld.config.txt; then
        echo "# adbex" >> /linkerconfig/com.android.adbd/ld.config.txt
        echo "namespace.default.permitted.paths += $ADBEX_PATH" >> /linkerconfig/com.android.adbd/ld.config.txt
        echo "zeromount: patched adbd linker config" > /dev/kmsg 2>/dev/null
    fi
fi

echo "zeromount: injecting adbex into init (PID 1)" > /dev/kmsg 2>/dev/null
"$INJECT" 1 "$ADBEX_PATH/libadbex_init.so"
INJECT_RC=$?

if [ "$INJECT_RC" -eq 0 ]; then
    echo "zeromount: adbex injection successful" > /dev/kmsg 2>/dev/null
else
    echo "zeromount: adbex injection failed (rc=$INJECT_RC)" > /dev/kmsg 2>/dev/null
fi
