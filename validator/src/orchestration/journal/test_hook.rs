use std::cell::RefCell;

thread_local! {
    static PRE_RENAME_HOOK: RefCell<Option<Box<dyn FnOnce()>>> =
        const { RefCell::new(None) };
    static POST_RENAME_HOOK: RefCell<Option<Box<dyn FnOnce()>>> =
        const { RefCell::new(None) };
}

pub(super) fn set(hook: impl FnOnce() + 'static) {
    PRE_RENAME_HOOK.with(|slot| {
        assert!(slot.borrow_mut().replace(Box::new(hook)).is_none());
    });
}

pub(super) fn run() {
    PRE_RENAME_HOOK.with(|slot| {
        if let Some(hook) = slot.borrow_mut().take() {
            hook();
        }
    });
}

pub(super) fn set_post_rename(hook: impl FnOnce() + 'static) {
    POST_RENAME_HOOK.with(|slot| {
        assert!(slot.borrow_mut().replace(Box::new(hook)).is_none());
    });
}

pub(super) fn run_post_rename() {
    POST_RENAME_HOOK.with(|slot| {
        if let Some(hook) = slot.borrow_mut().take() {
            hook();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::{run_post_rename, set_post_rename};
    use std::sync::atomic::{AtomicBool, Ordering};

    static RAN: AtomicBool = AtomicBool::new(false);

    #[test]
    fn post_rename_hook_runs_once() {
        set_post_rename(|| RAN.store(true, Ordering::SeqCst));
        run_post_rename();
        run_post_rename();
        assert!(RAN.swap(false, Ordering::SeqCst));
    }
}
