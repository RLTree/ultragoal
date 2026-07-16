use super::*;

#[path = "acquisition.rs"]
mod acquisition;
#[path = "cleanup.rs"]
mod cleanup;
#[path = "root.rs"]
mod root;
#[path = "snapshot.rs"]
mod snapshot;

pub(in crate::routine_work) use acquisition::{LaunchCleanupEvidence, ObservedLaunchCleanup};
pub(super) use acquisition::{fail_staged, observe_staged_cleanup};
#[cfg(test)]
pub(crate) use acquisition::{
    set_test_launch_cleanup_refusal, set_test_launch_panic_after_stat,
    set_test_launch_stat_failure_after,
};
pub(super) use snapshot::{LaunchBinding, launch_root, stage_program};
