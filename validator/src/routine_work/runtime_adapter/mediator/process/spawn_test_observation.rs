use super::*;

#[cfg(test)]
pub(crate) static TEST_SPAWN_COUNT: AtomicU64 = AtomicU64::new(0);

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
pub(crate) enum SetupFailurePoint {
    ProcessGroup,
    StdoutNonblocking,
    StderrNonblocking,
    StdoutReaderStart,
    StderrReaderStart,
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

/// Owns every spawned-process resource until all fallible setup completes.
///
/// It is installed immediately after `spawn()`. Any return or unwind before
/// `into_running` terminates and reaps the process group, closes unhanded pipe
/// descriptors, signals started readers to stop, and joins them.
pub(crate) struct SpawnSetupGuard {
    pub(crate) child: Option<Child>,
    pub(crate) process_group: Option<ProcessGroupId>,
    pub(crate) stdout: Option<ChildStdout>,
    pub(crate) stderr: Option<ChildStderr>,
    pub(crate) stdin: Option<ChildStdin>,
    pub(crate) readers_done: Arc<AtomicBool>,
    pub(crate) stdout_reader: Option<ReaderHandle>,
    pub(crate) stderr_reader: Option<ReaderHandle>,
    pub(crate) stdin_writer: Option<WriterHandle>,
    pub(crate) cleanup_required: bool,
}

impl SpawnSetupGuard {
    pub(crate) fn new(child: Child) -> Self {
        let process_group = ProcessGroupId::from_child_id(child.id()).ok();
        Self {
            child: Some(child),
            process_group,
            stdout: None,
            stderr: None,
            stdin: None,
            readers_done: Arc::new(AtomicBool::new(false)),
            stdout_reader: None,
            stderr_reader: None,
            stdin_writer: None,
            cleanup_required: true,
        }
    }

    pub(crate) fn child_id(&self) -> u32 {
        self.child.as_ref().expect("spawn setup retains child").id()
    }

    pub(crate) fn process_group(&self) -> Result<ProcessGroupId, RoutineError> {
        self.process_group
            .ok_or_else(|| mediator_error("mediator-process-group-invalid"))
    }

    pub(crate) fn take_pipes(&mut self) -> Result<(), RoutineError> {
        let child = self
            .child
            .as_mut()
            .expect("spawn guard always owns its child before handoff");
        self.stdout = Some(
            child
                .stdout
                .take()
                .ok_or_else(|| mediator_error("mediator-stdout-unavailable"))?,
        );
        self.stderr = Some(
            child
                .stderr
                .take()
                .ok_or_else(|| mediator_error("mediator-stderr-unavailable"))?,
        );
        self.stdin = child.stdin.take();
        Ok(())
    }

    pub(crate) fn into_running(mut self) -> RunningProcess {
        let running = RunningProcess {
            child: self.child.take(),
            process_group: self
                .process_group
                .take()
                .expect("validated process group before setup handoff"),
            readers_done: Arc::clone(&self.readers_done),
            stdout_reader: self.stdout_reader.take(),
            stderr_reader: self.stderr_reader.take(),
            stdin_writer: self.stdin_writer.take(),
            cleanup_required: true,
        };
        self.cleanup_required = false;
        running
    }
}

impl Drop for SpawnSetupGuard {
    fn drop(&mut self) {
        if !self.cleanup_required {
            return;
        }
        self.readers_done.store(true, Ordering::Release);
        if let Some(child) = self.child.as_mut() {
            let _ = cleanup_spawned_child(child, self.process_group);
        }
        self.stdout.take();
        self.stderr.take();
        self.stdin.take();
        join_reader_best_effort(&mut self.stdout_reader);
        join_reader_best_effort(&mut self.stderr_reader);
        join_writer_best_effort(&mut self.stdin_writer);
    }
}

/// Owns child, group, and reader threads for all post-setup fallible work.
/// Dropping it before `disarm` repeats fail-closed cleanup and joins readers.
pub(crate) struct RunningProcess {
    pub(crate) child: Option<Child>,
    pub(crate) process_group: ProcessGroupId,
    pub(crate) readers_done: Arc<AtomicBool>,
    pub(crate) stdout_reader: Option<ReaderHandle>,
    pub(crate) stderr_reader: Option<ReaderHandle>,
    pub(crate) stdin_writer: Option<WriterHandle>,
    pub(crate) cleanup_required: bool,
}

impl RunningProcess {
    pub(crate) fn child_mut(&mut self) -> &mut Child {
        self.child
            .as_mut()
            .expect("running process retains child ownership")
    }

    pub(crate) fn terminate_and_reap(&mut self) -> Result<(), RoutineError> {
        let process_group = self.process_group;
        terminate_and_reap(self.child_mut(), process_group)
    }

    pub(crate) fn join_io(&mut self) -> Result<(Drained, Drained), RoutineError> {
        self.readers_done.store(true, Ordering::Release);
        let stdout = self
            .stdout_reader
            .take()
            .expect("stdout reader installed before setup handoff")
            .join()
            .map_err(|_| mediator_error("mediator-stdout-reader-failed"))??;
        let stderr = self
            .stderr_reader
            .take()
            .expect("stderr reader installed before setup handoff")
            .join()
            .map_err(|_| mediator_error("mediator-stderr-reader-failed"))??;
        if let Some(writer) = self.stdin_writer.take() {
            writer
                .join()
                .map_err(|_| mediator_error("mediator-stdin-writer-failed"))??;
        }
        Ok((stdout, stderr))
    }

    pub(crate) fn disarm(&mut self) {
        debug_assert!(
            self.stdout_reader.is_none()
                && self.stderr_reader.is_none()
                && self.stdin_writer.is_none()
        );
        self.cleanup_required = false;
    }
}

impl Drop for RunningProcess {
    fn drop(&mut self) {
        if !self.cleanup_required {
            return;
        }
        self.readers_done.store(true, Ordering::Release);
        if let Some(child) = self.child.as_mut() {
            let _ = cleanup_spawned_child(child, Some(self.process_group));
        }
        join_reader_best_effort(&mut self.stdout_reader);
        join_reader_best_effort(&mut self.stderr_reader);
        join_writer_best_effort(&mut self.stdin_writer);
    }
}

pub(crate) fn join_reader_best_effort(reader: &mut Option<ReaderHandle>) {
    if let Some(reader) = reader.take() {
        let _ = reader.join();
    }
}

pub(crate) fn join_writer_best_effort(writer: &mut Option<WriterHandle>) {
    if let Some(writer) = writer.take() {
        let _ = writer.join();
    }
}
