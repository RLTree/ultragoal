use super::*;

#[cfg(test)]
pub(crate) type ProcessFailureHook = Box<dyn FnOnce() + Send + 'static>;

#[cfg(test)]
thread_local! {
    static PROCESS_FAILURE_HOOKS: RefCell<Vec<(ProcessFailurePoint, ProcessFailureHook)>> =
        RefCell::new(Vec::new());
}

#[cfg(test)]
pub(crate) fn set_test_process_failure(
    point: ProcessFailurePoint,
    hook: impl FnOnce() + Send + 'static,
) {
    PROCESS_FAILURE_HOOKS.with(|slot| {
        let mut slot = slot.borrow_mut();
        slot.clear();
        slot.push((point, Box::new(hook)));
    });
}

#[cfg(test)]
pub(crate) fn maybe_inject_process_failure(
    point: ProcessFailurePoint,
    cause: &'static str,
) -> Result<(), RoutineError> {
    let hook = PROCESS_FAILURE_HOOKS.with(|slot| {
        let mut slot = slot.borrow_mut();
        slot.iter()
            .position(|(expected, _)| *expected == point)
            .map(|index| slot.remove(index).1)
    });
    if let Some(hook) = hook {
        hook();
        return Err(mediator_error(cause));
    }
    Ok(())
}

#[cfg(not(test))]
pub(crate) fn maybe_inject_process_failure(
    _point: ProcessFailurePoint,
    _cause: &'static str,
) -> Result<(), RoutineError> {
    Ok(())
}
