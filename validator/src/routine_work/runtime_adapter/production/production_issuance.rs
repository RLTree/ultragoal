use super::ledger::{PendingRecovery, ReusePreauthorization};
use super::production_mediation::{
    allowed_output_scopes, error, now_tick, random_session_id, reservation_grant_id,
};
use super::*;

pub(super) struct RecoveryBinding {
    pub(super) marker: String,
    pub(super) output_journal: OutputProvisionJournal,
}

pub(super) fn validate_recovery(pending: PendingRecovery) -> Result<RecoveryBinding, RoutineError> {
    if pending.grant_id.is_empty() || now_tick()? > pending.deadline_tick {
        return Err(error("routine-production-recovery-authority-stale"));
    }
    Ok(RecoveryBinding {
        marker: pending.marker,
        output_journal: pending.output_journal,
    })
}

pub(super) fn reuse_claims(
    claims: Vec<super::super::mediator::ProductionReuseClaim>,
) -> Vec<ReuseArtifactClaim> {
    claims
        .into_iter()
        .map(|claim| ReuseArtifactClaim {
            protocol_id: claim.protocol_id,
            intent_id: claim.intent_id,
            artifact_sha256: claim.artifact_sha256,
            result_artifact_sha256: claim.result_artifact_sha256,
            mediator_witness_sha256: claim.mediator_witness_sha256,
        })
        .collect()
}

pub(super) fn reservation_spec(
    request: &RoutineEffectRequest,
    binding: AuthorityBinding,
    recovery_for: Option<String>,
    reuse_only: bool,
    reuse_preauthorization: Option<ReusePreauthorization>,
    output_journal: OutputProvisionJournal,
) -> Result<ReservationSpec, RoutineError> {
    let scopes = allowed_output_scopes(request);
    let session_id = random_session_id(&binding)?;
    let grant_id = reservation_grant_id(
        &session_id,
        request,
        &binding,
        &scopes,
        recovery_for.as_deref(),
    )?;
    Ok(ReservationSpec {
        binding,
        request_id: request.request_id().to_owned(),
        recovery_marker: recovery_identity(&grant_id, request.protocol_id(), request.request_id()),
        grant_id,
        recovery_for,
        reuse_only,
        reuse_preauthorization,
        output_journal,
    })
}

pub(super) fn output_journal(
    context: &LiveContext,
    request: &RoutineEffectRequest,
    recovery: Option<&RecoveryBinding>,
) -> Result<OutputProvisionJournal, RoutineError> {
    let scopes = allowed_output_scopes(request);
    let journal = match recovery {
        Some(recovery) => recovery.output_journal.clone(),
        None => super::output_journal::observe(context.worktree_root(), &scopes)?,
    };
    let expected = scopes
        .iter()
        .map(|scope| scope.as_str().to_owned())
        .collect::<Vec<_>>();
    (journal.scopes == expected)
        .then_some(journal)
        .ok_or_else(|| error("routine-production-output-journal-binding-stale"))
}
