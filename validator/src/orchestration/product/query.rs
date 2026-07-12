use super::context::{ReadOnlySink, open_engine};
use super::snapshot::snapshot;
use super::{ProductContext, ProductError, ProductSnapshot, ProductWorkspace};
use crate::orchestration::JournalHead;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QueryRequest {
    pub expected_head: JournalHead,
    pub tick: u64,
    pub live_workers: BTreeSet<String>,
}

/// Recursively read-only query over the anchored journal and replayed kernel.
pub fn query(
    context: &ProductContext,
    workspace: &ProductWorkspace,
    request: &QueryRequest,
) -> Result<ProductSnapshot, ProductError> {
    let engine = open_engine(context, workspace, &request.expected_head, ReadOnlySink)?;
    let result = snapshot(&engine, request.tick, &request.live_workers)?;
    workspace.verify()?;
    Ok(result)
}
