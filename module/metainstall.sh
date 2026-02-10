#!/system/bin/sh
# Runs when ANOTHER module is installed via KSU/APatch
MODDIR="${0%/*}"

KSU_HAS_METAMODULE=true
KSU_METAMODULE=meta-zeromount
export KSU_HAS_METAMODULE KSU_METAMODULE

handle_partition() { : ; }

install_module
