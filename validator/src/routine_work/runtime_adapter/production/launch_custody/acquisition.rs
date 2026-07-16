use super::*;

#[cfg(test)]
use std::cell::Cell;
use std::fs::File;

use crate::routine_work::runtime_adapter::mediator::{ObjectIdentity, StagedProgram};

use super::cleanup::{EntryClaim, cleanup_partial_stage};

pub(in crate::routine_work) struct ObservedLaunchCleanup {
    outcome: std::thread::Result<Result<(), RoutineError>>,
    evidence: LaunchCleanupEvidence,
}

#[derive(Eq, PartialEq)]
pub(in crate::routine_work) struct LaunchCleanupEvidence {
    evidence: CleanupEvidence,
}

impl LaunchCleanupEvidence {
    pub(in crate::routine_work) fn as_cleanup(&self) -> &CleanupEvidence {
        &self.evidence
    }

    pub(in crate::routine_work::runtime_adapter::production) fn into_cleanup(
        self,
    ) -> CleanupEvidence {
        self.evidence
    }
}

impl ObservedLaunchCleanup {
    fn capture(cleanup: impl FnOnce() -> Result<(), RoutineError>) -> Self {
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(cleanup));
        let evidence = match &outcome {
            Ok(Ok(())) => CleanupEvidence::Succeeded,
            Ok(Err(error)) => CleanupEvidence::Error(error.evidence()),
            Err(payload) => CleanupEvidence::Panic(PanicEvidence::capture(payload.as_ref())),
        };
        Self {
            outcome,
            evidence: LaunchCleanupEvidence { evidence },
        }
    }

    pub(in crate::routine_work) fn evidence(&self) -> &CleanupEvidence {
        self.evidence.as_cleanup()
    }

    pub(in crate::routine_work::runtime_adapter::production) fn with_transition(
        self,
        transition: std::thread::Result<Result<(), RoutineError>>,
    ) -> Self {
        if self.evidence() != &CleanupEvidence::Succeeded {
            return self;
        }
        let evidence = match &transition {
            Ok(Ok(())) => CleanupEvidence::Succeeded,
            Ok(Err(error)) => CleanupEvidence::Error(error.evidence()),
            Err(payload) => CleanupEvidence::Panic(PanicEvidence::capture(payload.as_ref())),
        };
        Self {
            outcome: transition,
            evidence: LaunchCleanupEvidence { evidence },
        }
    }

    pub(in crate::routine_work::runtime_adapter::production) fn into_parts(
        self,
    ) -> (
        std::thread::Result<Result<(), RoutineError>>,
        LaunchCleanupEvidence,
    ) {
        (self.outcome, self.evidence)
    }

    pub(in crate::routine_work) fn into_evidence(self) -> LaunchCleanupEvidence {
        self.evidence
    }
}

pub(in crate::routine_work::runtime_adapter::production) fn observe_staged_cleanup(
    staged: &StagedProgram,
) -> ObservedLaunchCleanup {
    ObservedLaunchCleanup::capture(|| super::cleanup::cleanup_staged(staged))
}

pub(in crate::routine_work::runtime_adapter::production) fn fail_staged(
    staged: &StagedProgram,
    primary: RoutineError,
) -> RoutineError {
    primary.with_launch_cleanup(observe_staged_cleanup(staged))
}

pub(super) struct LaunchAcquisitionCustody {
    child: PathBuf,
    directory: Option<File>,
    directory_identity: Option<ObjectIdentity>,
    entries: Vec<EntryClaim>,
}

impl LaunchAcquisitionCustody {
    pub(super) fn created(child: PathBuf) -> Self {
        Self {
            child,
            directory: None,
            directory_identity: None,
            entries: Vec::new(),
        }
    }

    pub(super) fn child(&self) -> &Path {
        &self.child
    }

    pub(super) fn hold_directory(&mut self, directory: File) {
        self.directory = Some(directory);
    }

    pub(super) fn duplicate_directory(&self) -> Result<File, RoutineError> {
        self.directory
            .as_ref()
            .ok_or_else(|| error("routine-production-launch-directory-custody-missing"))?
            .try_clone()
            .map_err(|_| error("routine-production-launch-directory-duplicate-failed"))
    }

    pub(super) fn bind_directory(&mut self, identity: ObjectIdentity) {
        self.directory_identity = Some(identity);
    }

    pub(super) fn claim(&mut self, claim: EntryClaim) {
        self.entries.push(claim);
    }

    pub(super) fn observe_cleanup(self) -> ObservedLaunchCleanup {
        ObservedLaunchCleanup::capture(move || {
            if refuse_cleanup() {
                return Err(error("routine-production-launch-cleanup-injected-refusal"));
            }
            let directory = match self.directory {
                Some(directory) => directory,
                None => File::open(&self.child)
                    .map_err(|_| error("routine-production-launch-directory-open-failed"))?,
            };
            let identity = match self.directory_identity {
                Some(identity) => identity,
                None => ObjectIdentity::from(
                    &directory
                        .metadata()
                        .map_err(|_| error("routine-production-launch-directory-stat-failed"))?,
                ),
            };
            cleanup_partial_stage(&self.child, identity, &self.entries)
        })
    }
}

#[cfg(test)]
thread_local! {
    static STAT_FAILURE: Cell<Option<usize>> = const { Cell::new(None) };
    static PANIC_AFTER_STAT: Cell<Option<usize>> = const { Cell::new(None) };
    static CLEANUP_REFUSAL: Cell<bool> = const { Cell::new(false) };
}

#[cfg(test)]
pub(crate) fn set_test_launch_stat_failure_after(observations: usize) {
    STAT_FAILURE.with(|slot| slot.set(Some(observations)));
}

#[cfg(test)]
pub(crate) fn set_test_launch_cleanup_refusal(refuse: bool) {
    CLEANUP_REFUSAL.with(|slot| slot.set(refuse));
}

#[cfg(test)]
pub(crate) fn set_test_launch_panic_after_stat(observations: usize) {
    PANIC_AFTER_STAT.with(|slot| slot.set(Some(observations)));
}

#[cfg(test)]
pub(super) fn observe_directory_stat() -> Result<(), RoutineError> {
    PANIC_AFTER_STAT.with(|slot| match slot.get() {
        Some(0) => {
            slot.set(None);
            panic!("routine-production-launch-acquisition-injected-panic");
        }
        Some(remaining) => slot.set(Some(remaining - 1)),
        None => {}
    });
    STAT_FAILURE.with(|slot| match slot.get() {
        Some(0) => {
            slot.set(None);
            Err(error("routine-production-launch-directory-stat-failed"))
        }
        Some(remaining) => {
            slot.set(Some(remaining - 1));
            Ok(())
        }
        None => Ok(()),
    })
}

#[cfg(not(test))]
pub(super) fn observe_directory_stat() -> Result<(), RoutineError> {
    Ok(())
}

#[cfg(test)]
fn refuse_cleanup() -> bool {
    CLEANUP_REFUSAL.with(|slot| slot.replace(false))
}

#[cfg(not(test))]
fn refuse_cleanup() -> bool {
    false
}
