use super::*;

#[path = "cleanup.rs"]
mod cleanup;
#[path = "root.rs"]
mod root;
#[path = "snapshot.rs"]
mod snapshot;

pub(in crate::routine_work) use cleanup::ObservedStagedCleanup;
pub(super) use cleanup::cleanup_staged;
pub(super) use cleanup::fail_staged;
pub(super) use snapshot::{LaunchBinding, launch_root, stage_program};
