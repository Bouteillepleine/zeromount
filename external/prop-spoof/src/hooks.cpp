#include <sys/system_properties.h>
#include <cstring>
#include <unistd.h>
#include <android/log.h>
#include "dobby.h"
#include "hooks.h"

#define LOG_TAG "PropSpoof"
#define LOGD(...) __android_log_print(ANDROID_LOG_DEBUG, LOG_TAG, __VA_ARGS__)

static const struct { const char* name; const char* value; } spoof_table[] = {
    { "init.svc.adbd",              "stopped" },
    { "init.svc_debug_pid.adbd",    ""        },
    { "sys.usb.config",             "mtp"     },
    { "sys.usb.state",              "mtp"     },
    { "sys.usb.ffs.ready",          "0"       },
    { "sys.usb.ffs.adb.ready",      "0"       },
    { "service.adb.root",           "0"       },
    { "service.adb.tcp.port",       "-1"      },
    { "persist.sys.usb.config",     "mtp"     },
    { "persist.service.adb.enable", "0"       },
    { "persist.adb.tcp.port",       ""        },
    { "persist.vendor.usb.config",  "none"    },
    { "vendor.usb.config",          "none"    },
};

static const char* lookup_spoof(const char* name) {
    for (auto& entry : spoof_table) {
        if (strcmp(entry.name, name) == 0)
            return entry.value;
    }
    return nullptr;
}

static void (*orig_read_callback)(const prop_info*, void (*)(void*, const char*, const char*, uint32_t), void*) = nullptr;
static int  (*orig_property_get)(const char*, char*) = nullptr;
static int  (*orig_property_read)(const prop_info*, char*, char*) = nullptr;

struct spoof_cookie {
    void (*real_cb)(void*, const char*, const char*, uint32_t);
    void* real_cookie;
};

static void spoof_cb(void* cookie, const char* name, const char* value, uint32_t serial) {
    auto* ctx = static_cast<spoof_cookie*>(cookie);
    const char* spoofed = lookup_spoof(name);
    ctx->real_cb(ctx->real_cookie, name, spoofed ? spoofed : value, serial);
}

static void hook_read_callback(
        const prop_info* pi,
        void (*cb)(void*, const char*, const char*, uint32_t),
        void* cookie)
{
    if (getuid() < 10000) {
        orig_read_callback(pi, cb, cookie);
        return;
    }
    spoof_cookie ctx = { cb, cookie };
    orig_read_callback(pi, spoof_cb, &ctx);
}

static int hook_property_get(const char* name, char* value) {
    if (getuid() < 10000)
        return orig_property_get(name, value);

    const char* spoof = lookup_spoof(name);
    if (spoof) {
        strlcpy(value, spoof, PROP_VALUE_MAX);
        return (int)strlen(spoof);
    }
    return orig_property_get(name, value);
}

static int hook_property_read(const prop_info* pi, char* name, char* value) {
    int ret = orig_property_read(pi, name, value);
    if (getuid() < 10000 || !name)
        return ret;

    const char* spoof = lookup_spoof(name);
    if (spoof && value) {
        strlcpy(value, spoof, PROP_VALUE_MAX);
        ret = (int)strlen(spoof);
    }
    return ret;
}

void install_property_hooks() {
    dobby_enable_near_branch_trampoline();

    void* sym_cb = DobbySymbolResolver("libc.so", "__system_property_read_callback");
    if (sym_cb)
        DobbyHook(sym_cb, (void*)hook_read_callback, (void**)&orig_read_callback);
    else
        LOGD("__system_property_read_callback not found");

    void* sym_get = DobbySymbolResolver("libc.so", "__system_property_get");
    if (sym_get)
        DobbyHook(sym_get, (void*)hook_property_get, (void**)&orig_property_get);
    else
        LOGD("__system_property_get not found");

    void* sym_read = DobbySymbolResolver("libc.so", "__system_property_read");
    if (sym_read)
        DobbyHook(sym_read, (void*)hook_property_read, (void**)&orig_property_read);
    else
        LOGD("__system_property_read not found");
}
