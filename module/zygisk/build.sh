#!/bin/sh
# Build the Zygisk ADB-hiding module: hooker.dex + arm64-v8a.so
# Requires: NDK r29, d8/dx (build-tools 34), javac, ANDROID_JAR

set -e

NDK="${NDK:-/opt/android-ndk-r29}"
BUILD_TOOLS="${BUILD_TOOLS:-$HOME/Android/Sdk/build-tools/34.0.0}"
ANDROID_JAR="${ANDROID_JAR:-$HOME/Android/Sdk/platforms/android-34/android.jar}"
CLANG="$NDK/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android24-clang++"
LSPLANT_ROOT="${LSPLANT_ROOT:-$(realpath ../../external/lsplant)}"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
CXA_ATEXIT_ROOT="$SCRIPT_DIR/external/cxa_atexit"
OUT="$SCRIPT_DIR/out"
mkdir -p "$OUT"

echo "=== Building hooker.dex ==="
JAVA_SRC="$SCRIPT_DIR/java"
CLASS_OUT="$OUT/classes"
mkdir -p "$CLASS_OUT"

javac -source 8 -target 8 \
    -bootclasspath "$ANDROID_JAR" \
    -classpath "$ANDROID_JAR" \
    -d "$CLASS_OUT" \
    "$JAVA_SRC/com/zeromount/hook/SettingsHooker.java"

"$BUILD_TOOLS/d8" \
    --output "$OUT" \
    --min-api 26 \
    "$CLASS_OUT/com/zeromount/hook/SettingsHooker.class"

mv "$OUT/classes.dex" "$OUT/hooker.dex"
echo "  -> $OUT/hooker.dex"

echo "=== Building arm64-v8a.so ==="

# LSPlant is compiled as a static library and linked in.
# lsplant.cc requires C++23 with module support — we include only the header
# and link against a pre-built liblsplant.a if available, otherwise stub it.
LSPLANT_INCLUDE="$LSPLANT_ROOT/lsplant/src/main/jni/include"

"$CLANG" \
    -shared -fPIC -O2 -std=c++17 \
    -fvisibility=hidden \
    -ffunction-sections -fdata-sections \
    -I "$SCRIPT_DIR/include" \
    -I "$LSPLANT_INCLUDE" \
    "$SCRIPT_DIR/src/main.cpp" \
    "$SCRIPT_DIR/src/settings_hook.cpp" \
    "$CXA_ATEXIT_ROOT/atexit.cpp" \
    -o "$OUT/arm64-v8a.so" \
    -llog -landroid \
    -Wl,--gc-sections \
    -Wl,--exclude-libs,ALL

echo "  -> $OUT/arm64-v8a.so"
echo "=== Done ==="
echo ""
echo "Deploy:"
echo "  cp $OUT/arm64-v8a.so  <module_zip>/zygisk/arm64-v8a.so"
echo "  cp $OUT/hooker.dex    <module_zip>/zygisk/hooker.dex"
