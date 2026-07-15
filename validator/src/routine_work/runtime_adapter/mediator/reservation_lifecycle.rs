use super::*;
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};

type PanicPayload = Box<dyn std::any::Any + Send>;

struct CapturedCleanup {
    outcome: std::thread::Result<Result<(), RoutineError>>,
    evidence: CleanupEvidence,
}

struct ReservationFailurePanic {
    resume: PanicPayload,
    evidence: ReservationFailureEvidence,
}

pub(crate) fn run_reserved<T>(
    attempt: AttemptReservation,
    lifecycle: impl FnOnce(&AttemptReservation) -> Result<T, RoutineError>,
) -> Result<T, RoutineError> {
    let outcome = catch_unwind(AssertUnwindSafe(|| lifecycle(&attempt)));
    match outcome {
        Ok(Ok(value)) if attempt.settled.get() => Ok(value),
        Ok(Ok(_)) => finish_missing_transition(&attempt),
        Ok(Err(error)) => finish_error(&attempt, error),
        Err(payload) => finish_panic(&attempt, payload),
    }
}

pub(super) fn observe_staged_transition(
    attempt: &AttemptReservation,
    operation: impl FnOnce() -> Result<(), RoutineError>,
) -> Result<(), RoutineError> {
    let primary = match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(Err(error)) if error.reservation_failure_evidence().is_some() => return Err(error),
        Err(payload) if payload.is::<ReservationFailurePanic>() => resume_unwind(payload),
        outcome => outcome,
    };
    let cleanup = capture_staged_cleanup(attempt);
    match primary {
        Ok(Ok(())) => finish_cleanup_observation(attempt, cleanup),
        Ok(Err(error)) => {
            let evidence = observed_error(attempt, &error, cleanup.evidence);
            drop(cleanup.outcome);
            Err(error.with_reservation_failure_evidence(evidence))
        }
        Err(payload) => {
            let (resume, primary, process_cleanup) = panic_observation(payload);
            let evidence = attempt.failure_evidence(primary, process_cleanup, cleanup.evidence);
            drop(cleanup.outcome);
            resume_unwind(Box::new(ReservationFailurePanic { resume, evidence }))
        }
    }
}

fn finish_missing_transition<T>(attempt: &AttemptReservation) -> Result<T, RoutineError> {
    let cleanup = capture_staged_cleanup(attempt);
    let evidence = cleanup.evidence.clone();
    let record = attempt.failure_evidence(
        FailureEvidence::MissingSettlement,
        CleanupEvidence::NotRequired,
        evidence,
    );
    record_transition(attempt, &record)?;
    match cleanup.outcome {
        Ok(Ok(())) => Err(mediator_error(
            "mediator-reservation-terminal-transition-missing",
        )),
        Ok(Err(error)) => Err(error),
        Err(payload) => resume_unwind(payload),
    }
}

fn finish_error<T>(attempt: &AttemptReservation, error: RoutineError) -> Result<T, RoutineError> {
    if let Some(evidence) = error.reservation_failure_evidence() {
        record_transition(attempt, evidence)?;
        return Err(error);
    }
    let (primary, process_cleanup) = error
        .process_custody()
        .map(|evidence| (evidence.primary.clone(), evidence.cleanup.clone()))
        .unwrap_or_else(|| {
            (
                FailureEvidence::Error(error.evidence()),
                CleanupEvidence::NotRequired,
            )
        });
    let cleanup = capture_staged_cleanup(attempt);
    let staged_cleanup = cleanup.evidence.clone();
    let record = attempt.failure_evidence(primary, process_cleanup, staged_cleanup);
    record_transition(attempt, &record)?;
    match cleanup.outcome {
        Err(payload) => resume_unwind(payload),
        Ok(_) => Err(error),
    }
}

fn finish_panic<T>(attempt: &AttemptReservation, payload: PanicPayload) -> Result<T, RoutineError> {
    let payload = match payload.downcast::<ReservationFailurePanic>() {
        Ok(failure) => {
            let ReservationFailurePanic { resume, evidence } = *failure;
            if let Err(error) = record_transition(attempt, &evidence) {
                drop(resume);
                return Err(error);
            }
            resume_unwind(resume)
        }
        Err(payload) => payload,
    };
    let (original, process) = match process::take_process_custody_panic(payload) {
        Ok((original, process)) => (original, process),
        Err(original) => {
            let process = ProcessCustodyEvidence {
                primary: FailureEvidence::Panic(PanicEvidence::capture(original.as_ref())),
                cleanup: CleanupEvidence::NotRequired,
            };
            (original, process)
        }
    };
    let cleanup = capture_staged_cleanup(attempt);
    let staged_cleanup = cleanup.evidence.clone();
    let record = attempt.failure_evidence(process.primary, process.cleanup, staged_cleanup);
    if let Err(error) = record_transition(attempt, &record) {
        drop(original);
        return Err(error);
    }
    resume_unwind(original)
}

fn finish_cleanup_observation(
    attempt: &AttemptReservation,
    cleanup: CapturedCleanup,
) -> Result<(), RoutineError> {
    match cleanup.outcome {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => {
            let evidence = observed_error(attempt, &error, cleanup.evidence);
            Err(error.with_reservation_failure_evidence(evidence))
        }
        Err(payload) => {
            let (resume, primary, process_cleanup) = panic_observation(payload);
            let evidence = attempt.failure_evidence(primary, process_cleanup, cleanup.evidence);
            resume_unwind(Box::new(ReservationFailurePanic { resume, evidence }))
        }
    }
}

fn observed_error(
    attempt: &AttemptReservation,
    error: &RoutineError,
    staged_cleanup: CleanupEvidence,
) -> ReservationFailureEvidence {
    let (primary, process_cleanup) = error
        .process_custody()
        .map(|evidence| (evidence.primary.clone(), evidence.cleanup.clone()))
        .unwrap_or_else(|| {
            (
                FailureEvidence::Error(error.evidence()),
                CleanupEvidence::NotRequired,
            )
        });
    attempt.failure_evidence(primary, process_cleanup, staged_cleanup)
}

fn panic_observation(payload: PanicPayload) -> (PanicPayload, FailureEvidence, CleanupEvidence) {
    match process::take_process_custody_panic(payload) {
        Ok((resume, evidence)) => (resume, evidence.primary, evidence.cleanup),
        Err(resume) => {
            let primary = FailureEvidence::Panic(PanicEvidence::capture(resume.as_ref()));
            (resume, primary, CleanupEvidence::NotRequired)
        }
    }
}

fn capture_staged_cleanup(attempt: &AttemptReservation) -> CapturedCleanup {
    let outcome = catch_unwind(AssertUnwindSafe(|| attempt.cleanup_staged()));
    let evidence = match &outcome {
        Ok(Ok(())) => CleanupEvidence::Succeeded,
        Ok(Err(error)) => CleanupEvidence::Error(error.evidence()),
        Err(payload) => CleanupEvidence::Panic(PanicEvidence::capture(payload.as_ref())),
    };
    CapturedCleanup { outcome, evidence }
}

fn record_transition(
    attempt: &AttemptReservation,
    record: &ReservationFailureEvidence,
) -> Result<(), RoutineError> {
    match catch_unwind(AssertUnwindSafe(|| {
        attempt.record_failure_and_transition(record)
    })) {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(transition_failure_error(
            "mediator-reservation-failure-transition-failed",
            record.clone(),
            FailureEvidence::Error(error.evidence()),
        )),
        Err(payload) => {
            let failure = FailureEvidence::Panic(PanicEvidence::capture(payload.as_ref()));
            Err(transition_failure_error(
                "mediator-reservation-failure-transition-panicked",
                record.clone(),
                failure,
            ))
        }
    }
}

#[cfg(test)]
#[path = "reservation_lifecycle/cleanup_panic_tests.rs"]
mod cleanup_panic_tests;
#[cfg(test)]
#[path = "reservation_lifecycle/failure_matrix_tests.rs"]
mod failure_matrix_tests;
#[cfg(test)]
#[path = "reservation_lifecycle/failure_transition_tests.rs"]
mod failure_transition_tests;
