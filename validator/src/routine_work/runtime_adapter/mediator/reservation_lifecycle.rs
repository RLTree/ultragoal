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

fn cleanup_and_transition(
    attempt: &AttemptReservation,
    process_cleanup_failure: Option<process::ProcessCleanupFailure>,
) -> std::thread::Result<Result<(), RoutineError>> {
    let cleanup = catch_unwind(AssertUnwindSafe(|| attempt.cleanup_staged()));
    attempt.transition_failure();
    drop(process_cleanup_failure);
    cleanup
}

pub(crate) fn run_reserved<T>(
    attempt: AttemptReservation,
    lifecycle: impl FnOnce(&AttemptReservation) -> Result<T, RoutineError>,
) -> Result<T, RoutineError> {
    let outcome = catch_unwind(AssertUnwindSafe(|| lifecycle(&attempt)));
    match outcome {
        Ok(Ok(value)) if attempt.settled.get() => Ok(value),
        Ok(Ok(_)) => match cleanup_and_transition(&attempt, None) {
            Ok(cleanup) => cleanup.and(Err(mediator_error(
                "mediator-reservation-terminal-transition-missing",
            ))),
            Err(payload) => resume_unwind(payload),
        },
        Ok(Err(error)) => match cleanup_and_transition(&attempt, None) {
            Ok(_) => Err(error),
            Err(payload) => resume_unwind(payload),
        },
        Err(payload) => {
            let (payload, process_cleanup_failure) =
                match process::take_process_custody_panic(payload) {
                    Ok((original, cleanup_failure)) => (original, Some(cleanup_failure)),
                    Err(original) => (original, None),
                };
            drop(cleanup_and_transition(&attempt, process_cleanup_failure));
            resume_unwind(payload)
        }
    }
}

#[cfg(test)]
#[path = "reservation_lifecycle/cleanup_panic_tests.rs"]
mod cleanup_panic_tests;
