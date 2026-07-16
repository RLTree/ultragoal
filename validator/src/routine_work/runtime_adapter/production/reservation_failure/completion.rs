use super::*;

pub(in crate::routine_work::runtime_adapter::production) fn finish_execution<T>(
    primary: std::thread::Result<Result<T, RoutineError>>,
    cleanup: ObservedLaunchCleanup,
) -> Result<T, RoutineError> {
    match primary {
        Ok(Ok(value)) => match combine(Ok(Ok(())), cleanup) {
            Ok(()) => Ok(value),
            Err(failure) => std::panic::resume_unwind(Box::new(failure)),
        },
        Ok(Err(error)) => resume(combine(Ok(Err(error)), cleanup)),
        Err(payload) => resume(combine(Err(payload), cleanup)),
    }
}

fn resume<T>(result: Result<(), LifecycleFailure>) -> Result<T, RoutineError> {
    match result {
        Ok(()) => Err(super::super::production_mediation::error(
            "routine-process-transition-missing",
        )),
        Err(failure) => std::panic::resume_unwind(Box::new(failure)),
    }
}

fn combine(
    primary: std::thread::Result<Result<(), RoutineError>>,
    cleanup: ObservedLaunchCleanup,
) -> Result<(), LifecycleFailure> {
    let (outcome, evidence) = cleanup.into_parts();
    match primary {
        Ok(Ok(())) => match outcome {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => observed_error(error, evidence),
            Err(payload) => observed_panic(payload, evidence),
        },
        Ok(Err(error)) => observed_error(error, evidence),
        Err(payload) => observed_panic(payload, evidence),
    }
}

fn observed_error(
    error: RoutineError,
    launch: LaunchCleanupEvidence,
) -> Result<(), LifecycleFailure> {
    let (primary, process_cleanup, observed_launch) = error_parts(&error);
    Err(LifecycleFailure::Error(
        error,
        FailureParts {
            primary,
            process_cleanup,
            launch_cleanup: observed_launch.or(Some(launch.into_cleanup())),
            output_cleanup: CleanupEvidence::NotRequired,
        },
    ))
}

fn observed_panic(
    payload: PanicPayload,
    launch: LaunchCleanupEvidence,
) -> Result<(), LifecycleFailure> {
    let (payload, primary, process_cleanup) = panic_parts(payload);
    Err(LifecycleFailure::Panic(
        payload,
        FailureParts {
            primary,
            process_cleanup,
            launch_cleanup: Some(launch.into_cleanup()),
            output_cleanup: CleanupEvidence::NotRequired,
        },
    ))
}

pub(in crate::routine_work::runtime_adapter::production) fn finish_error<T>(
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

pub(in crate::routine_work::runtime_adapter::production) fn finish_panic<T>(
    payload: PanicPayload,
    transition: Result<(), RoutineError>,
) -> Result<T, RoutineError> {
    if let Err(error) = transition {
        drop(payload);
        return Err(error);
    }
    std::panic::resume_unwind(payload)
}
