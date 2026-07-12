use std::cell::RefCell;

thread_local! {
    static PRE_RENAME_HOOK: RefCell<Option<Box<dyn FnOnce()>>> =
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
