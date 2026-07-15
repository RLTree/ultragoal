use std::cell::RefCell;
use std::ffi::CStr;
use std::os::fd::RawFd;

#[cfg(test)]
thread_local! {
    static BEFORE_FINAL_REMOVAL: RefCell<Option<Box<dyn FnMut(&CStr)>>> = RefCell::new(None);
    static BEFORE_ENTRY_REMOVAL: RefCell<Option<Box<dyn FnMut(RawFd, &CStr)>>> = RefCell::new(None);
}

#[cfg(test)]
pub(crate) fn set_before_final_removal(hook: Option<Box<dyn FnMut(&CStr)>>) {
    BEFORE_FINAL_REMOVAL.with(|slot| *slot.borrow_mut() = hook);
}

#[cfg(test)]
pub(crate) fn run_before_final_removal(name: &CStr) -> bool {
    BEFORE_FINAL_REMOVAL.with(|slot| {
        let mut hook = slot.borrow_mut().take();
        if let Some(hook) = hook.as_mut() {
            hook(name);
            true
        } else {
            false
        }
    })
}

#[cfg(test)]
pub(crate) fn set_before_entry_removal(hook: Option<Box<dyn FnMut(RawFd, &CStr)>>) {
    BEFORE_ENTRY_REMOVAL.with(|slot| *slot.borrow_mut() = hook);
}

#[cfg(test)]
pub(crate) fn run_before_entry_removal(directory: RawFd, name: &CStr) -> bool {
    BEFORE_ENTRY_REMOVAL.with(|slot| {
        let mut hook = slot.borrow_mut().take();
        if let Some(hook) = hook.as_mut() {
            hook(directory, name);
            true
        } else {
            false
        }
    })
}

#[cfg(not(test))]
pub(crate) fn run_before_final_removal(_name: &CStr) -> bool {
    false
}

#[cfg(not(test))]
pub(crate) fn run_before_entry_removal(_directory: RawFd, _name: &CStr) -> bool {
    false
}
