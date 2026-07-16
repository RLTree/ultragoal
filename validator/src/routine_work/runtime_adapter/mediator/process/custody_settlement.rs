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

    pub(crate) fn cleanup(mut self) -> Result<(), RoutineError> {
        cleanup_owned_process(ProcessCustody {
            child: &mut self.child,
            process_group: self.process_group,
            readers_done: &self.readers_done,
            pipes: [&mut self.stdout, &mut self.stderr, &mut self.stdin],
            readers: [&mut self.stdout_reader, &mut self.stderr_reader],
            writer: &mut self.stdin_writer,
        })
    }
}

impl RunningProcess {
    pub(crate) fn observe<T>(
        mut self,
        operation: impl FnOnce(&mut Self) -> Result<T, RoutineError>,
    ) -> Result<T, RoutineError> {
        match catch_unwind(AssertUnwindSafe(|| operation(&mut self))) {
            Ok(Ok(value)) => settle_running_success(self).map(|()| value),
            Ok(Err(error)) => cleanup_running_error(self, error),
            Err(payload) => cleanup_running_panic(self, payload),
        }
    }

    pub(crate) fn cleanup(mut self) -> Result<(), RoutineError> {
        let mut stdout = None;
        let mut stderr = None;
        let mut stdin = None;
        cleanup_owned_process(ProcessCustody {
            child: &mut self.child,
            process_group: Some(self.process_group),
            readers_done: &self.readers_done,
            pipes: [&mut stdout, &mut stderr, &mut stdin],
            readers: [&mut self.stdout_reader, &mut self.stderr_reader],
            writer: &mut self.stdin_writer,
        })
    }
}

fn cleanup_setup_error<T>(
    setup: SpawnSetupGuard,
    primary: RoutineError,
) -> Result<T, RoutineError> {
    cleanup_after_error(|| setup.cleanup(), primary)
}

fn cleanup_running_error<T>(
    running: RunningProcess,
    primary: RoutineError,
) -> Result<T, RoutineError> {
    cleanup_after_error(|| running.cleanup(), primary)
}

fn cleanup_setup_panic<T>(setup: SpawnSetupGuard, payload: Box<dyn std::any::Any + Send>) -> T {
    cleanup_after_panic(|| setup.cleanup(), payload)
}

fn cleanup_running_panic<T>(running: RunningProcess, payload: Box<dyn std::any::Any + Send>) -> T {
    cleanup_after_panic(|| running.cleanup(), payload)
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
        Ok(Err(cleanup_error)) => {
            let cleanup = CleanupEvidence::Error(cleanup_error.evidence());
            Err(
                primary.with_process_custody(ObservedProcessCustody::new(ProcessCustodyEvidence {
                    primary: primary_evidence,
                    cleanup,
                })),
            )
        }
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

fn settle_running_success(mut running: RunningProcess) -> Result<(), RoutineError> {
    let validation = (|| {
        let handles_closed = running.stdout_reader.is_none()
            && running.stderr_reader.is_none()
            && running.stdin_writer.is_none();
        let child_reaped = running
            .child
            .as_mut()
            .ok_or_else(|| mediator_error("mediator-child-custody-missing"))?
            .try_wait()
            .map_err(|_| mediator_error("mediator-process-reap-status-failed"))?
            .is_some();
        Ok(handles_closed && child_reaped && !process_group_exists(running.process_group)?)
    })();
    match validation {
        Ok(true) => Ok(()),
        Ok(false) => cleanup_running_error(
            running,
            mediator_error("mediator-process-success-custody-incomplete"),
        ),
        Err(error) => cleanup_running_error(running, error),
    }
}

struct ProcessCustody<'a> {
    child: &'a mut Option<BoundChild>,
    process_group: Option<ProcessGroupId>,
    readers_done: &'a AtomicBool,
    pipes: [&'a mut Option<File>; 3],
    readers: [&'a mut Option<ReaderHandle>; 2],
    writer: &'a mut Option<WriterHandle>,
}

fn cleanup_owned_process(custody: ProcessCustody<'_>) -> Result<(), RoutineError> {
    custody.readers_done.store(true, Ordering::Release);
    let mut failure = custody
        .child
        .as_mut()
        .ok_or_else(|| mediator_error("mediator-child-custody-missing"))
        .and_then(|child| cleanup_spawned_child(child, custody.process_group))
        .err();
    for pipe in custody.pipes {
        drop(pipe.take());
    }
    for reader in custody.readers {
        record_failure(&mut failure, join_reader(reader));
    }
    record_failure(&mut failure, join_writer(custody.writer));
    record_failure(
        &mut failure,
        maybe_inject_process_failure(
            ProcessFailurePoint::Cleanup,
            "mediator-process-cleanup-injected",
        ),
    );
    failure.map_or(Ok(()), Err)
}

fn record_failure(failure: &mut Option<RoutineError>, result: Result<(), RoutineError>) {
    if let Err(error) = result {
        failure.get_or_insert(error);
    }
}

fn join_reader(reader: &mut Option<ReaderHandle>) -> Result<(), RoutineError> {
    match reader.take() {
        Some(reader) => reader
            .join()
            .map_err(|_| mediator_error("mediator-output-reader-join-failed"))?
            .map(|_| ()),
        None => Ok(()),
    }
}

fn join_writer(writer: &mut Option<WriterHandle>) -> Result<(), RoutineError> {
    match writer.take() {
        Some(writer) => writer
            .join()
            .map_err(|_| mediator_error("mediator-input-writer-join-failed"))?,
        None => Ok(()),
    }
}
