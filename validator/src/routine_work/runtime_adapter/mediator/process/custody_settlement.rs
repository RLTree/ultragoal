use super::*;
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};

pub(crate) struct ProcessCustodyPanic {
    resume: Box<dyn std::any::Any + Send>,
    evidence: ProcessCustodyEvidence,
}

pub(in crate::routine_work) struct ObservedProcessCustody {
    evidence: ProcessCustodyEvidence,
}

impl ObservedProcessCustody {
    fn new(evidence: ProcessCustodyEvidence) -> Self {
        Self { evidence }
    }

    pub(in crate::routine_work) fn into_evidence(self) -> ProcessCustodyEvidence {
        self.evidence
    }
}

pub(crate) fn take_process_custody_panic(
    payload: Box<dyn std::any::Any + Send>,
) -> Result<(Box<dyn std::any::Any + Send>, ProcessCustodyEvidence), Box<dyn std::any::Any + Send>>
{
    payload
        .downcast::<ProcessCustodyPanic>()
        .map(|custody| (custody.resume, custody.evidence))
}

#[cfg(target_os = "macos")]
impl SpawnSetupGuard {
    pub(crate) fn configure<T>(
        mut self,
        operation: impl FnOnce(&mut Self) -> Result<T, RoutineError>,
    ) -> Result<(Self, T), RoutineError> {
        match catch_unwind(AssertUnwindSafe(|| operation(&mut self))) {
            Ok(Ok(value)) => Ok((self, value)),
            Ok(Err(error)) => cleanup_setup_error(self, error),
            Err(payload) => cleanup_setup_panic(self, payload),
        }
    }

    pub(crate) fn cleanup(&mut self) -> Result<(), RoutineError> {
        let Some(mut process) = self.process.take() else {
            return Ok(());
        };
        crate::process_custody::cleanup_process(process.pid, process.group(), false)
            .map_err(|_| mediator_error("mediator-process-cleanup-failed"))?;
        process.mark_settled();
        Ok(())
    }
}

#[cfg(target_os = "macos")]
fn cleanup_setup_error<T>(
    mut setup: SpawnSetupGuard,
    primary: RoutineError,
) -> Result<T, RoutineError> {
    cleanup_after_error(|| setup.cleanup(), primary)
}

#[cfg(target_os = "macos")]
fn cleanup_setup_panic<T>(setup: SpawnSetupGuard, payload: Box<dyn std::any::Any + Send>) -> T {
    cleanup_after_panic(
        || {
            let mut setup = setup;
            setup.cleanup()
        },
        payload,
    )
}

fn cleanup_after_error<T>(
    cleanup: impl FnOnce() -> Result<(), RoutineError>,
    primary: RoutineError,
) -> Result<T, RoutineError> {
    let primary_evidence = FailureEvidence::Error(primary.evidence());
    match catch_unwind(AssertUnwindSafe(cleanup)) {
        Ok(Ok(())) => Err(primary.with_process_custody(ObservedProcessCustody::new(
            ProcessCustodyEvidence {
                primary: primary_evidence,
                cleanup: CleanupEvidence::Succeeded,
            },
        ))),
        Ok(Err(cleanup_error)) => Err(primary.with_process_custody(ObservedProcessCustody::new(
            ProcessCustodyEvidence {
                primary: primary_evidence,
                cleanup: CleanupEvidence::Error(cleanup_error.evidence()),
            },
        ))),
        Err(cleanup_panic) => {
            let evidence = ProcessCustodyEvidence {
                primary: primary_evidence,
                cleanup: CleanupEvidence::Panic(PanicEvidence::capture(cleanup_panic.as_ref())),
            };
            resume_unwind(Box::new(ProcessCustodyPanic {
                resume: cleanup_panic,
                evidence,
            }))
        }
    }
}

#[cfg(target_os = "macos")]
fn cleanup_after_panic<T>(
    cleanup: impl FnOnce() -> Result<(), RoutineError>,
    original: Box<dyn std::any::Any + Send>,
) -> T {
    let primary = FailureEvidence::Panic(PanicEvidence::capture(original.as_ref()));
    let cleanup = match catch_unwind(AssertUnwindSafe(cleanup)) {
        Ok(Ok(())) => CleanupEvidence::Succeeded,
        Ok(Err(error)) => CleanupEvidence::Error(error.evidence()),
        Err(payload) => CleanupEvidence::Panic(PanicEvidence::capture(payload.as_ref())),
    };
    resume_unwind(Box::new(ProcessCustodyPanic {
        resume: original,
        evidence: ProcessCustodyEvidence { primary, cleanup },
    }))
}
