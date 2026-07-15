use std::cell::RefCell;
use std::ffi::CStr;

#[cfg(test)]
thread_local! {
    static BEFORE_FINAL_REMOVAL: RefCell<Option<Box<dyn FnMut(&CStr)>>> = RefCell::new(None);
}

#[cfg(test)]
pub(crate) fn set_before_final_removal(hook: Option<Box<dyn FnMut(&CStr)>>) {
    BEFORE_FINAL_REMOVAL.with(|slot| *slot.borrow_mut() = hook);
}

#[cfg(test)]
pub(crate) fn run_before_final_removal(name: &CStr) {
    BEFORE_FINAL_REMOVAL.with(|slot| {
        if let Some(hook) = slot.borrow_mut().as_mut() {
            hook(name);
        }
    });
}

#[cfg(not(test))]
pub(crate) fn run_before_final_removal(_name: &CStr) {}
