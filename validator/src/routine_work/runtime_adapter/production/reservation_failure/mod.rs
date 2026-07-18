use super::*;

#[path = "completion.rs"]
mod completion;
pub(super) use completion::{finish_error, finish_execution, finish_panic};

pub(super) type PanicPayload = Box<dyn std::any::Any + Send>;
pub(super) type CleanupOutcome = std::thread::Result<Result<(), RoutineError>>;

pub(super) struct FailureBinding<'a> {
    pub(super) protocol_id: &'a str,
    pub(super) grant_id: &'a str,
    pub(super) recovery_marker: &'a str,
}

pub(super) struct CapturedCleanup {
    pub(super) outcome: CleanupOutcome,
    pub(super) evidence: CleanupEvidence,
}

impl CapturedCleanup {
    pub(super) fn from(outcome: CleanupOutcome) -> Self {
        let evidence = match &outcome {
            Ok(Ok(())) => CleanupEvidence::Succeeded,
            Ok(Err(error)) => CleanupEvidence::Error(error.evidence()),
            Err(payload) => CleanupEvidence::Panic(PanicEvidence::capture(payload.as_ref())),
        };
        Self { outcome, evidence }
    }
}

pub(super) struct FailureParts {
    primary: FailureEvidence,
    process_cleanup: CleanupEvidence,
    launch_cleanup: Option<CleanupEvidence>,
    output_cleanup: CleanupEvidence,
}

pub(super) enum LifecycleFailure {
    Error(RoutineError, FailureParts),
    Panic(PanicPayload, FailureParts),
}

pub(super) enum PrimaryFailure {
    Error(RoutineError),
    Panic(PanicPayload),
}

pub(super) enum TransactionFailure {
    Error(RoutineError, FailureParts, Option<CleanupOutcome>),
    Panic(PanicPayload, FailureParts),
}

pub(super) fn error_parts(
    error: &RoutineError,
) -> (FailureEvidence, CleanupEvidence, Option<CleanupEvidence>) {
    let (primary, process) = error
        .process_custody()
        .map(|evidence| (evidence.primary.clone(), evidence.cleanup.clone()))
        .unwrap_or_else(|| {
            (
                FailureEvidence::Error(error.evidence()),
                CleanupEvidence::NotRequired,
            )
        });
    (
        primary,
        process,
        error
            .launch_cleanup()
            .map(|observation| observation.as_cleanup().clone()),
    )
}

pub(super) fn resume_launch_acquisition_panic(
    payload: PanicPayload,
    cleanup: ObservedLaunchCleanup,
) -> ! {
    let primary = FailureEvidence::Panic(PanicEvidence::capture(payload.as_ref()));
    let (_, cleanup) = cleanup.into_parts();
    std::panic::resume_unwind(Box::new(LifecycleFailure::Panic(
        payload,
        FailureParts {
            primary,
            process_cleanup: CleanupEvidence::NotRequired,
            launch_cleanup: Some(cleanup.into_cleanup()),
            output_cleanup: CleanupEvidence::NotRequired,
        },
    )))
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

pub(super) fn observed_transition(
    record: &ReservationFailureEvidence,
    outcome: std::thread::Result<Result<(), RoutineError>>,
) -> Result<(), RoutineError> {
    match outcome {
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

pub(super) fn failure_record(
    binding: FailureBinding<'_>,
    started: bool,
    parts: FailureParts,
) -> ReservationFailureEvidence {
    let staged_cleanup = combined_cleanup(parts.launch_cleanup.as_ref(), &parts.output_cleanup);
    ReservationFailureEvidence {
        schema_version: RESERVATION_FAILURE_SCHEMA.to_owned(),
        protocol_id: binding.protocol_id.to_owned(),
        grant_id: binding.grant_id.to_owned(),
        recovery_marker: binding.recovery_marker.to_owned(),
        primary: parts.primary,
        process_cleanup: parts.process_cleanup,
        staged_cleanup,
        disposition: if started {
            ReservationFailureDisposition::StartedPending
        } else {
            ReservationFailureDisposition::ReservedPending
        },
    }
}

pub(super) fn parts_from_error(error: &RoutineError, cleanup: &CapturedCleanup) -> FailureParts {
    let (primary, process_cleanup, launch_cleanup) = error_parts(error);
    FailureParts {
        primary,
        process_cleanup,
        launch_cleanup,
        output_cleanup: cleanup.evidence.clone(),
    }
}

pub(super) fn observe(primary: PrimaryFailure, cleanup: CapturedCleanup) -> TransactionFailure {
    match primary {
        PrimaryFailure::Error(error) => {
            let parts = parts_from_error(&error, &cleanup);
            TransactionFailure::Error(error, parts, Some(cleanup.outcome))
        }
        PrimaryFailure::Panic(payload) => match payload.downcast::<LifecycleFailure>() {
            Ok(failure) => match *failure {
                LifecycleFailure::Error(error, parts) => {
                    TransactionFailure::Error(error, merge_cleanup(parts, cleanup), None)
                }
                LifecycleFailure::Panic(payload, parts) => {
                    TransactionFailure::Panic(payload, merge_cleanup(parts, cleanup))
                }
            },
            Err(payload) => {
                let (payload, primary, process_cleanup) = panic_parts(payload);
                TransactionFailure::Panic(
                    payload,
                    FailureParts {
                        primary,
                        process_cleanup,
                        launch_cleanup: None,
                        output_cleanup: cleanup.evidence,
                    },
                )
            }
        },
    }
}

fn merge_cleanup(mut parts: FailureParts, cleanup: CapturedCleanup) -> FailureParts {
    parts.output_cleanup = cleanup.evidence;
    parts
}

fn combined_cleanup(launch: Option<&CleanupEvidence>, output: &CleanupEvidence) -> CleanupEvidence {
    if let Some(value @ (CleanupEvidence::Error(_) | CleanupEvidence::Panic(_))) = launch {
        return value.clone();
    }
    if matches!(
        output,
        CleanupEvidence::Error(_) | CleanupEvidence::Panic(_)
    ) {
        return output.clone();
    }
    launch.cloned().unwrap_or_else(|| output.clone())
}
