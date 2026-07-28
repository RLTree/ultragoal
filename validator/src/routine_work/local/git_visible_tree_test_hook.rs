#[cfg(test)]
use std::cell::RefCell;

#[cfg(test)]
thread_local! {
    static BEFORE_DIRECTORY_OPEN: RefCell<Option<Box<dyn FnOnce()>>> = RefCell::new(None);
}

#[cfg(test)]
pub(super) fn set(hook: impl FnOnce() + 'static) {
    BEFORE_DIRECTORY_OPEN.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}

#[cfg(test)]
pub(super) fn run() {
    if let Some(hook) = BEFORE_DIRECTORY_OPEN.with(|slot| slot.borrow_mut().take()) {
        hook();
    }
}

#[cfg(not(test))]
pub(super) fn run() {}
