use super::context::{ReadOnlySink, journal_head_identity};
use super::snapshot::snapshot;
use super::{
    PermitTarget, ProductContext, ProductError, ProductSnapshot, ProductWorkspace, RootAuthority,
    RootOperation, RootPermit,
};
use crate::orchestration::{Binding, FileJournal, JournalHead, Orchestrator};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoverRequest {
    pub expected_prior_head: JournalHead,
    pub expected_event_id: String,
    pub recovered_binding: Binding,
    pub tick: u64,
    pub live_workers: BTreeSet<String>,
    pub target: PermitTarget,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecoverOutcome {
    pub schema_version: String,
    pub workspace_identity: String,
    pub workspace_path_current_after_commit: bool,
    pub prior_head: JournalHead,
    pub recovered_head: JournalHead,
    pub snapshot: ProductSnapshot,
}

pub fn recover(
    context: &ProductContext,
    workspace: &ProductWorkspace,
    authority: &RootAuthority,
    permit: &RootPermit,
    request: &RecoverRequest,
) -> Result<RecoverOutcome, ProductError> {
    if request.target.operation_id.as_deref() != Some(&request.expected_event_id)
        || request.target.recovered_binding.as_ref() != Some(&request.recovered_binding)
    {
        return Err(ProductError::AuthorityOperationMismatch);
    }
    let head_identity = journal_head_identity(&request.expected_prior_head)?;
    authority.verify(
        permit,
        &context.root,
        RootOperation::Recover,
        &context.binding,
        workspace.identity(),
        &head_identity,
        request.tick,
        &request.target,
    )?;
    workspace.verify()?;
    let prepared = FileJournal::prepare_interrupted_append(
        workspace.root(),
        &request.expected_prior_head,
        &request.expected_event_id,
        &request.recovered_binding,
    )
    .map_err(ProductError::from)?;
    let prospective = prepared.prospective().clone();
    if prospective.head.binding != request.recovered_binding {
        return Err(ProductError::StaleCandidate);
    }
    let effective_context = ProductContext::new(
        context.graph.clone(),
        context.policy.clone(),
        request.recovered_binding.clone(),
        context.root.clone(),
    );
    let preview = Orchestrator::restart_verified_snapshot(
        effective_context.graph.clone(),
        effective_context.policy.clone(),
        effective_context.root.clone(),
        prospective.clone(),
        ReadOnlySink,
    )
    .map_err(ProductError::from)?;
    let current = snapshot(&preview, request.tick, &request.live_workers)?;
    let mut snapshot_target = request.target.clone();
    snapshot_target.operation_id = None;
    current.require_target(&snapshot_target)?;
    if !current.recovery.ambiguous_operations.is_empty()
        || current.recovery.pending_integration_id.is_some()
    {
        return Err(ProductError::AmbiguousRecovery);
    }
    if !current.recovery.stale_binding_leases.is_empty() {
        return Err(ProductError::StaleCandidate);
    }
    if !current.recovery.expired_leases.is_empty() {
        return Err(ProductError::LeaseExpired);
    }
    if !current.recovery.orphaned_leases.is_empty() {
        return Err(ProductError::UnknownWorker);
    }
    workspace.verify()?;
    let repaired = prepared.commit().map_err(ProductError::from)?;
    let workspace_path_current_after_commit = workspace.verify().is_ok();
    Ok(RecoverOutcome {
        schema_version: "OrchestrationRecoverOutcome-v1".to_owned(),
        workspace_identity: workspace.identity().to_owned(),
        workspace_path_current_after_commit,
        prior_head: request.expected_prior_head.clone(),
        recovered_head: repaired.head,
        snapshot: current,
    })
}
