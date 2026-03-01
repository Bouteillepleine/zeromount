#include <unistd.h>
#include <android/log.h>

#include "zygisk.hpp"
#include "settings_hook.h"

#define TAG "ZeroMount-ADBHide"
#define LOGE(...) __android_log_print(ANDROID_LOG_ERROR, TAG, __VA_ARGS__)
#define LOGI(...) __android_log_print(ANDROID_LOG_INFO,  TAG, __VA_ARGS__)

class AdbHideModule : public zygisk::ModuleBase {
public:
    void onLoad(zygisk::Api *api, JNIEnv *env) override {
        api_ = api;
        env_ = env;
    }

    void preAppSpecialize(zygisk::AppSpecializeArgs *args) override {
        api_->setOption(zygisk::DLCLOSE_MODULE_LIBRARY);
    }

    void postAppSpecialize(const zygisk::AppSpecializeArgs *) override {}

    void preServerSpecialize(zygisk::ServerSpecializeArgs *) override {}

    void postServerSpecialize(const zygisk::ServerSpecializeArgs *) override {
        LOGI("postServerSpecialize: installing SettingsProvider hook");

        if (install_settings_hook(env_)) {
            LOGI("SettingsProvider hook installed");
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
