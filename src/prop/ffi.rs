use std::ffi::{c_char, c_int, c_void};

#[repr(C)]
pub struct PropInfo {
    _opaque: [u8; 0],
}

unsafe impl Send for PropInfo {}
unsafe impl Sync for PropInfo {}

extern "C" {
    pub fn __system_property_find(name: *const c_char) -> *const PropInfo;
    pub fn __system_property_serial(pi: *const PropInfo) -> u32;
    pub fn __system_property_set(name: *const c_char, value: *const c_char) -> c_int;

    // API 26+ — blocks on futex until property area serial changes
    pub fn __system_property_wait(
        pi: *const PropInfo,
        old_serial: u32,
        new_serial: *mut u32,
        timeout: *const libc::timespec,
    ) -> bool;

    pub fn __system_property_read_callback(
        pi: *const PropInfo,
        callback: unsafe extern "C" fn(*mut c_void, *const c_char, *const c_char, u32),
        cookie: *mut c_void,
    );
}
