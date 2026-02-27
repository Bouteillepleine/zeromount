package com.zeromount.hook;

import java.lang.reflect.Method;

/**
 * Hooker object for LSPlant. Holds the backup method reference and
 * exposes the native callback that LSPlant routes the hooked call through.
 *
 * LSPlant contract: callback must be `public Object methodName(Object[] args)`.
 * The native implementation is registered in settings_hook.cpp via RegisterNatives.
 */
public class SettingsHooker {
    public Method backup;

    public native Object getStringForUser(Object[] args);
}
