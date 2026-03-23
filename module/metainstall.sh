#!/system/bin/sh
# Runs when ANOTHER module is installed via KSU/APatch
MODDIR="${0%/*}"

KSU_HAS_METAMODULE=true
KSU_METAMODULE=meta-zeromount
export KSU_HAS_METAMODULE KSU_METAMODULE

# KSU install_module calls this callback — stub to suppress default partition handling
handle_partition() { : ; }

install_module

# KSU sets system_file during its own install flow, but the metamodule
# installer can race with restorecon. Belt-and-suspenders: force correct
# context so font/overlay modules work without reinstall.
if [ -d "$MODPATH/system" ] && command -v chcon >/dev/null 2>&1; then
    chcon -R u:object_r:system_file:s0 "$MODPATH/system" 2>/dev/null
fi

metamodule_hot_install() {
	[ "$KSU" = true ] || return
	[ -n "$MODID" ] || return

	MODDIR_LIVE="/data/adb/modules/$MODID"
	MODPATH_STAGED="/data/adb/modules_update/$MODID"
	[ -d "$MODDIR_LIVE" ] && [ -d "$MODPATH_STAGED" ] || return

	busybox rm -rf "$MODDIR_LIVE"
	busybox mv "$MODPATH_STAGED" "$MODDIR_LIVE"

	if [ -n "$MODULE_HOT_RUN_SCRIPT" ] && [ -f "$MODDIR_LIVE/$MODULE_HOT_RUN_SCRIPT" ]; then
		sh "$MODDIR_LIVE/$MODULE_HOT_RUN_SCRIPT"
	fi

	# stub satisfies KSU's ensure_file_exists check
	mkdir -p "$MODPATH_STAGED"
	cat "$MODDIR_LIVE/module.prop" > "$MODPATH_STAGED/module.prop"

	( sleep 3; rm -rf "$MODDIR_LIVE/update" "$MODPATH_STAGED" ) &

	ui_print "- Module hot-installed, no reboot needed!"
	ui_print "- Refresh module list to see changes."
}

[ "$MODULE_HOT_INSTALL_REQUEST" = true ] && metamodule_hot_install
