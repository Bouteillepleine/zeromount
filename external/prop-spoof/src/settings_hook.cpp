#include "settings_hook.h"
#include <android/log.h>

#define LOG_TAG "PropSpoof"
#define LOGD(...) __android_log_print(ANDROID_LOG_DEBUG, LOG_TAG, __VA_ARGS__)

void install_settings_hook(JNIEnv*) {
    // V1/V2/V3 handled via `settings put` in service.sh — see blueprint Section 3 Option A
    LOGD("Settings.Global hook deferred to Phase 2 (using settings put)");
}
