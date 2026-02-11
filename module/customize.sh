#!/system/bin/sh
# ZeroMount install hook — runs during KSU/APatch module installation.
SKIPUNZIP=1

unzip -o "$ZIPFILE" -d "$MODPATH" >&2

ZM_VERSION=$(grep '^version=' "$MODPATH/module.prop" | cut -d= -f2)
ui_print "- Installing ZeroMount ${ZM_VERSION}"

. "$MODPATH/common.sh"
if [ -z "$ABI" ]; then
    abort "! Unsupported architecture: $(uname -m)"
fi

ui_print "- Architecture: $ABI"

BIN="$MODPATH/bin/${ABI}/zeromount"

if [ ! -f "$BIN" ]; then
    abort "! Binary not found: bin/${ABI}/zeromount"
fi

chmod 755 "$MODPATH/bin/${ABI}"/*

for d in "$MODPATH"/bin/*/; do
    [ "$d" = "$MODPATH/bin/${ABI}/" ] && continue
    rm -rf "$d"
done

cp "$BIN" "$MODPATH/bin/zm"
chmod 755 "$MODPATH/bin/zm"

ZM_DATA="/data/adb/zeromount"
mkdir -p "$ZM_DATA"
mkdir -p "$ZM_DATA/logs"

if [ ! -f "$ZM_DATA/config.toml" ]; then
    ui_print "- Writing default config.toml"
    "$BIN" config defaults > "$ZM_DATA/config.toml" 2>/dev/null || true
fi

# Align SUSFS config with our defaults so both modules agree on first boot
SUSFS_CONFIG="/data/adb/susfs4ksu/config.sh"
if [ -f "$SUSFS_CONFIG" ]; then
    ui_print "- Syncing SUSFS config"
    sed -i 's/^susfs_log=.*/susfs_log=0/' "$SUSFS_CONFIG"
    sed -i 's/^avc_log_spoofing=.*/avc_log_spoofing=1/' "$SUSFS_CONFIG"
    sed -i 's/^hide_sus_mnts_for_all_or_non_su_procs=.*/hide_sus_mnts_for_all_or_non_su_procs=1/' "$SUSFS_CONFIG"
    sed -i 's/^emulate_vold_app_data=.*/emulate_vold_app_data=1/' "$SUSFS_CONFIG"
    sed -i 's/^force_hide_lsposed=.*/force_hide_lsposed=1/' "$SUSFS_CONFIG"
fi

ksud module config set manage.kernel_umount false 2>/dev/null || true

echo 0 > "$ZM_DATA/.bootcount"

if command -v chcon >/dev/null 2>&1; then
    chcon -R u:object_r:system_file:s0 "$MODPATH" 2>/dev/null || true
    chcon -R u:object_r:adb_data_file:s0 "$ZM_DATA" 2>/dev/null || true
fi

# Prevent nested webroot from stale installs
rm -rf "$MODPATH/webroot/webroot" 2>/dev/null

# Skip webroot/ — KSU handles its perms
set_perm_recursive "$MODPATH/bin" 0 0 0755 0755
chmod 755 "$MODPATH"/*.sh
set_perm "$MODPATH/module.prop" 0 0 0644

ui_print "- ZeroMount installed successfully"
