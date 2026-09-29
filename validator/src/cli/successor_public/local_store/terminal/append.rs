use super::super::LocalStoreFailure;
use crate::cli::successor_public::routine::{ContinuationCheckpoint, HostState};
use crate::context::LiveContext;
use std::path::Path;

pub(in super::super::super) fn append_routine_terminal(
    state: &HostState,
    target: &Path,
    context: &LiveContext,
    checkpoint: &ContinuationCheckpoint,
) -> Result<bool, LocalStoreFailure> {
    state
        .append_terminal_event(target, context, checkpoint)
        .map_err(|_| LocalStoreFailure::append("observe-routine-event-store-failed"))
}
