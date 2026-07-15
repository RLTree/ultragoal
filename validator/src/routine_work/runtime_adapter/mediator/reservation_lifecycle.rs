use super::read_source_binding::release_active;
use super::*;
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};

impl AttemptReservation {
    fn transition_failure(&self) {
        if self.settled.get() {
            return;
        }
        let mut state = registry()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        release_active(&mut state, &self.protocol_id, &self.grant_id);
        if self.started.get() && !state.ambiguous_protocols.contains_key(&self.protocol_id) {
            state
                .ambiguous_protocols
                .insert(self.protocol_id.clone(), self.recovery_marker.clone());
        }
        self.settled.set(true);
    }
}

pub(crate) fn run_reserved<T>(
    attempt: AttemptReservation,
    lifecycle: impl FnOnce(&AttemptReservation) -> Result<T, RoutineError>,
) -> Result<T, RoutineError> {
    let outcome = catch_unwind(AssertUnwindSafe(|| lifecycle(&attempt)));
    match outcome {
        Ok(Ok(value)) if attempt.settled.get() => Ok(value),
        Ok(Ok(_)) => {
            let cleanup = attempt.cleanup_staged();
            attempt.transition_failure();
            cleanup.and(Err(mediator_error(
                "mediator-reservation-terminal-transition-missing",
            )))
        }
        Ok(Err(error)) => {
            let cleanup = attempt.cleanup_staged();
            attempt.transition_failure();
            cleanup.and(Err(error))
        }
        Err(payload) => {
            let cleanup = attempt.cleanup_staged();
            attempt.transition_failure();
            match cleanup {
                Ok(()) => resume_unwind(payload),
                Err(error) => resume_unwind(Box::new(error)),
            }
        }
    }
}
