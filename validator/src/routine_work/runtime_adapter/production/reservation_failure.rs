use super::*;

pub(super) type PanicPayload = Box<dyn std::any::Any + Send>;

pub(super) struct CapturedCleanup {
    pub(super) outcome: std::thread::Result<Result<(), RoutineError>>,
    pub(super) evidence: CleanupEvidence,
}

impl CapturedCleanup {
    pub(super) fn from(outcome: std::thread::Result<Result<(), RoutineError>>) -> Self {
        let evidence = match &outcome {
            Ok(Ok(())) => CleanupEvidence::Succeeded,
            Ok(Err(error)) => CleanupEvidence::Error(error.evidence()),
            Err(payload) => CleanupEvidence::Panic(PanicEvidence::capture(payload.as_ref())),
        };
        Self { outcome, evidence }
    }
}

pub(super) struct FailureParts {
    pub(super) primary: FailureEvidence,
    pub(super) process_cleanup: CleanupEvidence,
    pub(super) staged_cleanup: CleanupEvidence,
}

pub(super) enum LifecycleFailure {
    Error(RoutineError, FailureParts),
    Panic(PanicPayload, FailureParts),
}

pub(super) fn combine(
    primary: std::thread::Result<Result<(), RoutineError>>,
    cleanup: CapturedCleanup,
) -> Result<(), LifecycleFailure> {
    match primary {
        Ok(Ok(())) => match cleanup.outcome {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => observed_error(error, cleanup.evidence),
            Err(payload) => observed_panic(payload, cleanup.evidence),
        },
        Ok(Err(error)) => observed_error(error, cleanup.evidence),
        Err(payload) => observed_panic(payload, cleanup.evidence),
    }
}

fn observed_error(error: RoutineError, staged: CleanupEvidence) -> Result<(), LifecycleFailure> {
    let (primary, process_cleanup) = error_parts(&error);
    Err(LifecycleFailure::Error(
        error,
        FailureParts {
            primary,
            process_cleanup,
            staged_cleanup: staged,
        },
    ))
}

fn observed_panic(payload: PanicPayload, staged: CleanupEvidence) -> Result<(), LifecycleFailure> {
    let (payload, primary, process_cleanup) = panic_parts(payload);
    Err(LifecycleFailure::Panic(
        payload,
        FailureParts {
            primary,
            process_cleanup,
            staged_cleanup: staged,
        },
    ))
}

pub(super) fn error_parts(error: &RoutineError) -> (FailureEvidence, CleanupEvidence) {
    error
        .process_custody()
        .map(|evidence| (evidence.primary.clone(), evidence.cleanup.clone()))
        .unwrap_or_else(|| {
            (
                FailureEvidence::Error(error.evidence()),
                CleanupEvidence::NotRequired,
            )
        })
}

pub(super) fn panic_parts(
    payload: PanicPayload,
) -> (PanicPayload, FailureEvidence, CleanupEvidence) {
    match super::super::mediator::take_process_custody_panic(payload) {
        Ok((resume, evidence)) => (resume, evidence.primary, evidence.cleanup),
        Err(resume) => {
            let primary = FailureEvidence::Panic(PanicEvidence::capture(resume.as_ref()));
            (resume, primary, CleanupEvidence::NotRequired)
        }
    }
}

pub(super) fn transition_result(
    record: &ReservationFailureEvidence,
    operation: impl FnOnce() -> Result<(), RoutineError>,
) -> Result<(), RoutineError> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(operation)) {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(transition_failure_error(
            "mediator-reservation-failure-transition-failed",
            record.clone(),
            FailureEvidence::Error(error.evidence()),
        )),
        Err(payload) => Err(transition_failure_error(
            "mediator-reservation-failure-transition-panicked",
            record.clone(),
            FailureEvidence::Panic(PanicEvidence::capture(payload.as_ref())),
        )),
    }
}

pub(super) fn finish_error<T>(
    error: RoutineError,
    cleanup: Option<std::thread::Result<Result<(), RoutineError>>>,
    transition: Result<(), RoutineError>,
) -> Result<T, RoutineError> {
    transition?;
    match cleanup {
        Some(Err(payload)) => std::panic::resume_unwind(payload),
        _ => Err(error),
    }
}

pub(super) fn finish_panic<T>(
    payload: PanicPayload,
    transition: Result<(), RoutineError>,
) -> Result<T, RoutineError> {
    if let Err(error) = transition {
        drop(payload);
        return Err(error);
    }
    std::panic::resume_unwind(payload)
}

pub(super) fn finish_observed<T>(
    failure: LifecycleFailure,
    transition: impl FnOnce(&FailureParts) -> Result<(), RoutineError>,
) -> Result<T, RoutineError> {
    match failure {
        LifecycleFailure::Error(error, parts) => finish_error(error, None, transition(&parts)),
        LifecycleFailure::Panic(payload, parts) => finish_panic(payload, transition(&parts)),
    }
}

pub(super) fn finish_transaction<T>(
    outcome: std::thread::Result<Result<T, RoutineError>>,
    token: &ReservationToken,
    started: bool,
    cleanup: impl FnOnce() -> CapturedCleanup,
    transition: impl FnOnce(&ReservationFailureEvidence) -> Result<(), RoutineError>,
) -> Result<T, RoutineError> {
    let evidence = |parts: FailureParts| ReservationFailureEvidence {
        schema_version: RESERVATION_FAILURE_SCHEMA.to_owned(),
        protocol_id: token.binding.protocol_id.clone(),
        grant_id: token.grant_id.clone(),
        recovery_marker: token.recovery_marker.clone(),
        primary: parts.primary,
        process_cleanup: parts.process_cleanup,
        staged_cleanup: parts.staged_cleanup,
        disposition: if started {
            ReservationFailureDisposition::StartedPending
        } else {
            ReservationFailureDisposition::ReservedPending
        },
    };
    match outcome {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(error)) => {
            let (primary, process_cleanup) = error_parts(&error);
            let cleanup = cleanup();
            let record = evidence(FailureParts {
                primary,
                process_cleanup,
                staged_cleanup: cleanup.evidence.clone(),
            });
            finish_error(error, Some(cleanup.outcome), transition(&record))
        }
        Err(payload) => match payload.downcast::<LifecycleFailure>() {
            Ok(failure) => finish_observed(*failure, |parts| {
                transition(&evidence(FailureParts {
                    primary: parts.primary.clone(),
                    process_cleanup: parts.process_cleanup.clone(),
                    staged_cleanup: parts.staged_cleanup.clone(),
                }))
            }),
            Err(payload) => {
                let (payload, primary, process_cleanup) = panic_parts(payload);
                let cleanup = cleanup();
                let record = evidence(FailureParts {
                    primary,
                    process_cleanup,
                    staged_cleanup: cleanup.evidence,
                });
                finish_panic(payload, transition(&record))
            }
        },
    }
}
