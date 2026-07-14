use super::context::{ReadOnlySink, journal_head_identity, open_engine};
use super::snapshot::snapshot;
use super::{
    PermitTarget, ProductContext, ProductError, ProductWorkspace, RootAuthority, RootPermit,
    RootReconcilePermitVerification,
};
use crate::orchestration::{EffectResolution, JournalHead};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReconcileRequest {
    pub expected_head: JournalHead,
    pub tick: u64,
    pub live_workers: BTreeSet<String>,
    pub lease_id: String,
    pub resolution: EffectResolution,
    pub target: PermitTarget,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReconcileOutcome {
    pub schema_version: String,
    pub prior_head: JournalHead,
    pub current_head: JournalHead,
    pub settled_operation_id: String,
    pub snapshot_id: String,
}

pub fn reconcile(
    context: &ProductContext,
    workspace: &ProductWorkspace,
    authority: &RootAuthority,
    permit: &RootPermit,
    request: &ReconcileRequest,
) -> Result<ReconcileOutcome, ProductError> {
    request
        .resolution
        .validate_shape()
        .map_err(ProductError::from)?;
    if request.target.lease_id.as_deref() != Some(&request.lease_id)
        || request.target.operation_id.as_deref() != Some(&request.resolution.operation_id)
    {
        return Err(ProductError::AuthorityOperationMismatch);
    }
    let head_identity = journal_head_identity(&request.expected_head)?;
    authority.verify_reconcile(RootReconcilePermitVerification {
        permit,
        expected_root: &context.root,
        binding: &context.binding,
        workspace_identity: workspace.identity(),
        journal_head_identity: &head_identity,
        tick: request.tick,
        target: &request.target,
        resolution: &request.resolution,
    })?;
    let mut engine = open_engine(context, workspace, &request.expected_head, ReadOnlySink)?;
    let before = snapshot(&engine, request.tick, &request.live_workers)?;
    before.require_target(&request.target)?;
    if !before
        .recovery
        .ambiguous_operations
        .contains(&request.resolution.operation_id)
    {
        return Err(ProductError::UnknownOperation);
    }
    engine
        .reconcile_effect(request.tick, &request.lease_id, request.resolution.clone())
        .map_err(ProductError::from)?;
    let after = snapshot(&engine, request.tick, &request.live_workers)?;
    if after
        .recovery
        .ambiguous_operations
        .contains(&request.resolution.operation_id)
        || !after
            .settled_operations
            .contains(&request.resolution.operation_id)
    {
        return Err(ProductError::AmbiguousRecovery);
    }
    workspace.verify()?;
    Ok(ReconcileOutcome {
        schema_version: "OrchestrationReconcileOutcome-v1".to_owned(),
        prior_head: request.expected_head.clone(),
        current_head: after.journal_head.clone(),
        settled_operation_id: request.resolution.operation_id.clone(),
        snapshot_id: after.snapshot_id()?,
    })
}
