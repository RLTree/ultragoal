#[cfg(test)]
use std::cell::RefCell;

use crate::context::LiveContext;

use super::{RoutineBinding, RoutineError, RoutineErrorId};

pub(crate) fn current_binding(context: &LiveContext) -> Result<RoutineBinding, RoutineError> {
    context.revalidate().map_err(|_| {
        RoutineError::new(
            RoutineErrorId::ConcurrentMutation,
            "live-context-revalidation-failed",
            None,
        )
    })?;
    RoutineBinding::from_live(context)
}

pub(crate) fn require_current_binding(
    context: &LiveContext,
    expected: &RoutineBinding,
    cause: &'static str,
) -> Result<RoutineBinding, RoutineError> {
    let current = current_binding(context)?;
    if &current != expected {
        return Err(RoutineError::new(
            RoutineErrorId::ContextMismatch,
            cause,
            None,
        ));
    }
    Ok(current)
}

pub(crate) fn ensure_unchanged(
    context: &LiveContext,
    initial: &RoutineBinding,
    cause: &'static str,
) -> Result<(), RoutineError> {
    if current_binding(context)? != *initial {
        return Err(RoutineError::new(
            RoutineErrorId::ConcurrentMutation,
            cause,
            None,
        ));
    }
    Ok(())
}

#[cfg(test)]
pub(crate) fn test_authority_checkpoint() {
    TEST_AUTHORITY_HOOK.with(|slot| {
        if let Some(hook) = slot.borrow_mut().take() {
            hook();
        }
    });
}

#[cfg(test)]
thread_local! {
    static TEST_AUTHORITY_HOOK: RefCell<Option<Box<dyn FnOnce()>>> = RefCell::new(None);
}

#[cfg(test)]
pub(crate) fn set_test_live_authority_hook(hook: impl FnOnce() + 'static) {
    TEST_AUTHORITY_HOOK.with(|slot| {
        let prior = slot.borrow_mut().replace(Box::new(hook));
        assert!(
            prior.is_none(),
            "test live-authority hook already installed"
        );
    });
}
