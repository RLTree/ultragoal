use super::*;

#[cfg(test)]
thread_local! {
    static TEST_LAST_SPAWN_GROUP: Cell<Option<ProcessGroupId>> = const { Cell::new(None) };
}

pub(crate) struct ProcessObservation {
    pub(crate) termination: ProcessTermination,
    pub(crate) stdout: Vec<u8>,
    pub(crate) stderr_sha256: String,
    pub(crate) output_byte_length: u64,
    pub(crate) started: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ProcessTermination {
    Exited(i32),
    Signaled(i32),
    TimedOut,
    Cancelled,
    OutputLimit,
    DescendantSurvived,
    CleanupFailed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ProcessFailurePoint {
    ProcessGroup,
    StdoutNonblocking,
    StderrNonblocking,
    StdoutReaderStart,
    StderrReaderStart,
    StdinWriterStart,
    Resume,
    Wait,
    StdoutJoin,
    StderrJoin,
    StdinJoin,
    Cleanup,
}

pub(crate) type ReaderHandle = JoinHandle<Result<Drained, RoutineError>>;
pub(crate) type WriterHandle = JoinHandle<Result<(), RoutineError>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ProcessGroupId(i32);

impl ProcessGroupId {
    pub(crate) fn from_child_id(child_id: u32) -> Result<Self, RoutineError> {
        let value = i32::try_from(child_id)
            .map_err(|_| mediator_error("mediator-process-group-invalid"))?;
        Self::new(value)
    }

    pub(crate) fn new(value: i32) -> Result<Self, RoutineError> {
        if value <= 0 || value.checked_neg().is_none() {
            return Err(mediator_error("mediator-process-group-invalid"));
        }
        Ok(Self(value))
    }

    pub(crate) fn signal_target(self) -> Result<i32, RoutineError> {
        self.0
            .checked_neg()
            .filter(|target| *target < 0)
            .ok_or_else(|| mediator_error("mediator-process-group-invalid"))
    }
}

/// Owns every spawned-process resource until explicit setup settlement.
pub(crate) struct SpawnSetupGuard {
    pub(crate) child: Option<BoundChild>,
    pub(crate) process_group: Option<ProcessGroupId>,
    pub(crate) stdout: Option<File>,
    pub(crate) stderr: Option<File>,
    pub(crate) stdin: Option<File>,
    pub(crate) readers_done: Arc<AtomicBool>,
    pub(crate) stdout_reader: Option<ReaderHandle>,
    pub(crate) stderr_reader: Option<ReaderHandle>,
    pub(crate) stdin_writer: Option<WriterHandle>,
}

impl SpawnSetupGuard {
    pub(crate) fn new(spawned: SpawnedProcess) -> Self {
        let process_group = ProcessGroupId::from_child_id(spawned.child.id()).ok();
        #[cfg(test)]
        TEST_LAST_SPAWN_GROUP.with(|slot| slot.set(process_group));
        Self {
            child: Some(spawned.child),
            process_group,
            stdout: Some(spawned.stdout),
            stderr: Some(spawned.stderr),
            stdin: Some(spawned.stdin),
            readers_done: Arc::new(AtomicBool::new(false)),
            stdout_reader: None,
            stderr_reader: None,
            stdin_writer: None,
        }
    }

    pub(crate) fn process_group(&self) -> Result<ProcessGroupId, RoutineError> {
        self.process_group
            .ok_or_else(|| mediator_error("mediator-process-group-invalid"))
    }

    pub(crate) fn child(&self) -> Result<&BoundChild, RoutineError> {
        self.child
            .as_ref()
            .ok_or_else(|| mediator_error("mediator-child-custody-missing"))
    }

    pub(crate) fn into_running(mut self, process_group: ProcessGroupId) -> RunningProcess {
        RunningProcess {
            child: self.child.take(),
            process_group,
            readers_done: Arc::clone(&self.readers_done),
            stdout_reader: self.stdout_reader.take(),
            stderr_reader: self.stderr_reader.take(),
            stdin_writer: self.stdin_writer.take(),
        }
    }
}

#[cfg(test)]
pub(crate) fn test_last_spawn_group() -> Option<ProcessGroupId> {
    TEST_LAST_SPAWN_GROUP.with(Cell::get)
}

/// Owns child, group, and I/O threads until explicit process settlement.
pub(crate) struct RunningProcess {
    pub(crate) child: Option<BoundChild>,
    pub(crate) process_group: ProcessGroupId,
    pub(crate) readers_done: Arc<AtomicBool>,
    pub(crate) stdout_reader: Option<ReaderHandle>,
    pub(crate) stderr_reader: Option<ReaderHandle>,
    pub(crate) stdin_writer: Option<WriterHandle>,
}

impl RunningProcess {
    pub(crate) fn child_mut(&mut self) -> Result<&mut BoundChild, RoutineError> {
        self.child
            .as_mut()
            .ok_or_else(|| mediator_error("mediator-child-custody-missing"))
    }

    pub(crate) fn resume(&self) -> Result<(), RoutineError> {
        maybe_inject_process_failure(
            ProcessFailurePoint::Resume,
            "mediator-process-resume-injected",
        )?;
        signal_group(self.process_group, libc::SIGCONT)
    }

    pub(crate) fn terminate_and_reap(&mut self) -> Result<(), RoutineError> {
        let process_group = self.process_group;
        terminate_and_reap(self.child_mut()?, process_group)
    }

    pub(crate) fn join_io(&mut self) -> Result<(Drained, Drained), RoutineError> {
        self.readers_done.store(true, Ordering::Release);
        maybe_inject_process_failure(
            ProcessFailurePoint::StdoutJoin,
            "mediator-stdout-reader-join-injected",
        )?;
        let stdout = self
            .stdout_reader
            .take()
            .ok_or_else(|| mediator_error("mediator-stdout-reader-missing"))?
            .join()
            .map_err(|_| mediator_error("mediator-stdout-reader-failed"))??;
        maybe_inject_process_failure(
            ProcessFailurePoint::StderrJoin,
            "mediator-stderr-reader-join-injected",
        )?;
        let stderr = self
            .stderr_reader
            .take()
            .ok_or_else(|| mediator_error("mediator-stderr-reader-missing"))?
            .join()
            .map_err(|_| mediator_error("mediator-stderr-reader-failed"))??;
        maybe_inject_process_failure(
            ProcessFailurePoint::StdinJoin,
            "mediator-stdin-writer-join-injected",
        )?;
        self.stdin_writer
            .take()
            .ok_or_else(|| mediator_error("mediator-stdin-writer-missing"))?
            .join()
            .map_err(|_| mediator_error("mediator-stdin-writer-failed"))??;
        Ok((stdout, stderr))
    }
}
