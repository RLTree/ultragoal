use crate::orchestration::product::snapshot::snapshot;
use crate::orchestration::product::{
    open_engine, ProductContext, ProductError, ProductWorkspace, ReadOnlySink, ReconcileOutcome,
    ReconcileRequest,
};

pub(super) fn execute(
    context: &ProductContext,
    workspace: &ProductWorkspace,
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
