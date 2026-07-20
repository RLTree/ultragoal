use super::*;

#[cfg(target_os = "macos")]
use std::cell::Cell;

#[cfg(target_os = "macos")]
thread_local! {
    static TEST_LAST_SPAWN_GROUP: Cell<Option<ProcessGroupId>> = const { Cell::new(None) };
}

pub(crate) struct ProcessObservation {
    pub(crate) termination: ProcessTermination,
    pub(crate) stdout: Vec<u8>,
    pub(crate) stderr_sha256: String,
    pub(crate) output_byte_length: u64,
}

pub(crate) struct StartedProcessIdentity {
    process_id: i32,
    process_group_id: i32,
}

impl StartedProcessIdentity {
    pub(super) fn new(process_id: i32, group: ProcessGroupId) -> Self {
        Self {
            process_id,
            process_group_id: group.0,
        }
    }

    pub(crate) fn process_id(&self) -> i32 {
        self.process_id
    }

    pub(crate) fn process_group_id(&self) -> i32 {
        self.process_group_id
    }
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

    #[cfg(test)]
    pub(crate) fn signal_target(self) -> Result<i32, RoutineError> {
        self.0
            .checked_neg()
            .filter(|target| *target < 0)
            .ok_or_else(|| mediator_error("mediator-process-group-invalid"))
    }

    #[cfg(test)]
    pub(crate) fn raw(self) -> i32 {
        self.0
    }
}

#[cfg(target_os = "macos")]
pub(crate) struct SpawnSetupGuard {
    pub(super) process: Option<crate::process_custody::DarwinSuspendedProcess>,
    pub(super) process_group: ProcessGroupId,
}

#[cfg(target_os = "macos")]
impl SpawnSetupGuard {
    pub(crate) fn new(spawned: SpawnedProcess) -> Self {
        let process_group = ProcessGroupId::from_child_id(spawned.process.group() as u32)
            .expect("Darwin process group must be the positive child pid");
        TEST_LAST_SPAWN_GROUP.with(|slot| slot.set(Some(process_group)));
        Self {
            process: Some(spawned.process),
            process_group,
        }
    }

    pub(crate) fn process_group(&self) -> Result<ProcessGroupId, RoutineError> {
        Ok(self.process_group)
    }

    pub(crate) fn process_id(&self) -> Result<i32, RoutineError> {
        self.process
            .as_ref()
            .map(|process| process.pid)
            .ok_or_else(|| mediator_error("mediator-child-custody-missing"))
    }

    pub(crate) fn validate_loaded(&self, program: &PinnedExecutable) -> Result<(), RoutineError> {
        use std::os::unix::fs::MetadataExt;
        let metadata = program
            .file
            .metadata()
            .map_err(|_| mediator_error("mediator-executable-metadata-failed"))?;
        self.process
            .as_ref()
            .ok_or_else(|| mediator_error("mediator-child-custody-missing"))?
            .validate_loaded_vnode(program.path(), metadata.dev(), metadata.ino())
            .map_err(|_| mediator_error("mediator-loaded-executable-unobserved"))
    }

    pub(crate) fn take_process(
        &mut self,
    ) -> Result<crate::process_custody::DarwinSuspendedProcess, RoutineError> {
        self.process
            .take()
            .ok_or_else(|| mediator_error("mediator-child-custody-missing"))
    }

    #[cfg(test)]
    pub(crate) fn into_running(mut self, process_group: ProcessGroupId) -> RunningProcess {
        RunningProcess {
            process: self.process.take(),
            _process_group: process_group,
        }
    }
}

#[cfg(all(target_os = "macos", test))]
impl Drop for SpawnSetupGuard {
    fn drop(&mut self) {
        let _ = self.process.take();
    }
}

#[cfg(target_os = "macos")]
#[cfg(test)]
pub(crate) fn test_last_spawn_group() -> Option<ProcessGroupId> {
    TEST_LAST_SPAWN_GROUP.with(Cell::get)
}

#[cfg(target_os = "macos")]
#[cfg(test)]
pub(crate) struct RunningProcess {
    pub(super) process: Option<crate::process_custody::DarwinSuspendedProcess>,
    pub(crate) _process_group: ProcessGroupId,
}

#[cfg(target_os = "macos")]
#[cfg(test)]
impl Drop for RunningProcess {
    fn drop(&mut self) {
        let _ = self.process.take();
    }
}
