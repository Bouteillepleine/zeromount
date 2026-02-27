#pragma once
#include <jni.h>
#include <stdbool.h>

// Returns true if the SettingsProvider hook was installed successfully.
// Must be called from postServerSpecialize context (system_server, UID 1000).
bool install_settings_hook(JNIEnv *env);
