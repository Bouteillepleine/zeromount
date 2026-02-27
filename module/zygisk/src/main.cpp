#include <fcntl.h>
#include <unistd.h>
#include <string.h>
#include <android/log.h>

#include "zygisk.hpp"
#include "settings_hook.h"

#define TAG "ZeroMount-ADBHide"
#define LOGE(...) __android_log_print(ANDROID_LOG_ERROR, TAG, __VA_ARGS__)
#define LOGI(...) __android_log_print(ANDROID_LOG_INFO,  TAG, __VA_ARGS__)
#define LOGD(...) __android_log_print(ANDROID_LOG_DEBUG, TAG, __VA_ARGS__)

// Sentinel written by zeromount Rust binary when hide_usb_debugging=true
#define SENTINEL_PATH "/data/adb/zeromount/flags/hide_usb_debugging"

static bool hiding_enabled() {
    return access(SENTINEL_PATH, F_OK) == 0;
}

class AdbHideModule : public zygisk::ModuleBase {
public:
    void onLoad(zygisk::Api *api, JNIEnv *env) override {
        api_ = api;
        env_ = env;
    }

    // Only system_server is our target.
    // We do nothing for app processes — the hook lives in system_server's address space.
    void preAppSpecialize(zygisk::AppSpecializeArgs *args) override {
        api_->setOption(zygisk::DLCLOSE_MODULE_LIBRARY);
    }

    void postAppSpecialize(const zygisk::AppSpecializeArgs *) override {}

    void preServerSpecialize(zygisk::ServerSpecializeArgs *) override {}

    void postServerSpecialize(const zygisk::ServerSpecializeArgs *) override {
        if (!hiding_enabled()) {
            LOGD("sentinel absent — ADB hiding disabled, unloading");
            api_->setOption(zygisk::DLCLOSE_MODULE_LIBRARY);
            return;
        }

        LOGI("postServerSpecialize: installing SettingsProvider hook");

        if (install_settings_hook(env_)) {
            LOGI("SettingsProvider hook installed");
            // Library must stay loaded — the hook callback lives in this .so
        } else {
            LOGE("SettingsProvider hook failed — unloading");
            api_->setOption(zygisk::DLCLOSE_MODULE_LIBRARY);
        }
    }

private:
    zygisk::Api *api_ = nullptr;
    JNIEnv      *env_ = nullptr;
};

REGISTER_ZYGISK_MODULE(AdbHideModule)
