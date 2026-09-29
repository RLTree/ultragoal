use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

static TEST_PREOPEN_PAUSE_MS: AtomicU64 = AtomicU64::new(0);
static TEST_PREOPEN_PAUSED: AtomicBool = AtomicBool::new(false);

pub fn set_test_preopen_pause_ms(milliseconds: u64) {
    TEST_PREOPEN_PAUSED.store(false, Ordering::SeqCst);
    TEST_PREOPEN_PAUSE_MS.store(milliseconds, Ordering::SeqCst);
}

pub fn test_preopen_is_paused() -> bool {
    TEST_PREOPEN_PAUSED.load(Ordering::SeqCst)
}

pub(super) fn pause_before_file_open() {
    let milliseconds = TEST_PREOPEN_PAUSE_MS.swap(0, Ordering::SeqCst);
    if milliseconds == 0 {
        return;
    }
    TEST_PREOPEN_PAUSED.store(true, Ordering::SeqCst);
    std::thread::sleep(std::time::Duration::from_millis(milliseconds));
}
