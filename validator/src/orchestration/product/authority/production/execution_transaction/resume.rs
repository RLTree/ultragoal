use crate::orchestration::product::snapshot::snapshot;
use crate::orchestration::product::{
    ProductContext, ProductError, ProductWorkspace, ReadOnlySink, ResumeOutcome, ResumeRequest,
    open_engine,
};

pub(super) fn execute(
    context: &ProductContext,
    workspace: &ProductWorkspace,
    request: &ResumeRequest,
) -> Result<ResumeOutcome, ProductError> {
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
