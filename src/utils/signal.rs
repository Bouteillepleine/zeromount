use std::sync::atomic::{AtomicBool, Ordering};

static SHUTDOWN_REQUESTED: AtomicBool = AtomicBool::new(false);

#[inline]
pub fn shutdown_requested() -> bool {
    SHUTDOWN_REQUESTED.load(Ordering::Acquire)
}

// SA_RESTART so blocked syscalls resume instead of returning EINTR on
// every signal — our loops recheck the flag each iteration regardless.
pub fn register_shutdown_handler() {
    extern "C" fn handler(_sig: libc::c_int) {
        SHUTDOWN_REQUESTED.store(true, Ordering::Release);
    }

    unsafe {
        let mut action: libc::sigaction = std::mem::zeroed();
        action.sa_sigaction = handler as *const () as libc::sighandler_t;
        libc::sigemptyset(&mut action.sa_mask);
        action.sa_flags = libc::SA_RESTART;

        libc::sigaction(libc::SIGTERM, &action, std::ptr::null_mut());
        libc::sigaction(libc::SIGINT, &action, std::ptr::null_mut());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shutdown_flag_round_trip() {
        SHUTDOWN_REQUESTED.store(true, Ordering::Release);
        assert!(shutdown_requested());
        SHUTDOWN_REQUESTED.store(false, Ordering::Release);
        assert!(!shutdown_requested());
    }
}
