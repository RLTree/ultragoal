use super::context::{ReadOnlySink, journal_head_identity, open_engine};
use super::snapshot::snapshot;
use super::{
    PermitTarget, ProductContext, ProductError, ProductSnapshot, ProductWorkspace, RootAuthority,
    RootOperation, RootPermit,
};
use crate::orchestration::JournalHead;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResumeRequest {
    pub expected_head: JournalHead,
    pub tick: u64,
    pub live_workers: BTreeSet<String>,
    pub target: PermitTarget,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResumeOutcome {
    pub schema_version: String,
    pub prior_head: JournalHead,
    pub current_head: JournalHead,
    pub root_recovered: bool,
    pub snapshot: ProductSnapshot,
}

pub fn resume(
    context: &ProductContext,
    workspace: &ProductWorkspace,
    authority: &RootAuthority,
    permit: &RootPermit,
    request: &ResumeRequest,
) -> Result<ResumeOutcome, ProductError> {
    let head_identity = journal_head_identity(&request.expected_head)?;
    authority.verify_action(
        permit,
        &context.root,
        RootOperation::Resume,
        &context.binding,
        workspace.identity(),
        &head_identity,
        request.tick,
        &request.target,
    )?;
    let mut engine = open_engine(context, workspace, &request.expected_head, ReadOnlySink)?;
    let before = snapshot(&engine, request.tick, &request.live_workers)?;
    before.require_target(&request.target)?;
    if !before.recovery.ambiguous_operations.is_empty()
        || before.recovery.pending_integration_id.is_some()
    {
        return Err(ProductError::AmbiguousRecovery);
    }
    if !before.recovery.stale_binding_leases.is_empty() {
        return Err(ProductError::StaleCandidate);
    }
    if !before.recovery.expired_leases.is_empty() {
        return Err(ProductError::LeaseExpired);
    }
    if !before.recovery.orphaned_leases.is_empty() {
        return Err(ProductError::UnknownWorker);
    }
    let root_recovered = before.recovery.interrupted_root;
    if root_recovered {
        engine
            .recover_root(request.tick)
            .map_err(ProductError::from)?;
    }
    let after = snapshot(&engine, request.tick, &request.live_workers)?;
    after.require_target(&request.target)?;
    workspace.verify()?;
    Ok(ResumeOutcome {
        schema_version: "OrchestrationResumeOutcome-v1".to_owned(),
        prior_head: request.expected_head.clone(),
        current_head: after.journal_head.clone(),
        root_recovered,
        snapshot: after,
    })
}
