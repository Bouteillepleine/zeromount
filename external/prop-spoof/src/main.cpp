#include "zygisk.hpp"
#include "dobby.h"
#include "hooks.h"
#include "settings_hook.h"

class PropSpoofModule : public zygisk::ModuleBase {
public:
    void onLoad(zygisk::Api* api, JNIEnv* env) override {
        this->api = api;
        this->env = env;
        install_property_hooks();
    }

    void preServerSpecialize(zygisk::ServerSpecializeArgs* args) override {}

    void postServerSpecialize(const zygisk::ServerSpecializeArgs* args) override {
        install_settings_hook(env);
    }

private:
    zygisk::Api* api = nullptr;
    JNIEnv* env = nullptr;
};

REGISTER_ZYGISK_MODULE(PropSpoofModule)
