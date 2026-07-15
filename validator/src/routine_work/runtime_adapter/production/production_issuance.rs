use super::launch_recovery::recover_staged;
use super::launch_snapshot::launch_root;
use super::production_mediation::{
    DurableAttempt, allowed_output_scopes, authority_binding, error, now_tick, random_session_id,
};
use super::recovery_authority::RoutineRecoveryAuthority;
use super::*;

impl ProductionRoutineIssuer {
    /// Opens an explicit, pre-existing, owner-only authority root. This is an
    /// effectful constructor and must never be called by help/read/query paths.
    pub(super) fn open(authority_root: &Path) -> Result<Self, RoutineError> {
        let launch_root = launch_root(authority_root)?;
        FileAuthorityLedger::open_or_initialize(authority_root).map(|ledger| Self {
            ledger: Arc::new(ledger),
            launch_root,
        })
    }

    pub(super) fn open_existing(authority_root: &Path) -> Result<Self, RoutineError> {
        let launch_root = launch_root(authority_root)?;
        FileAuthorityLedger::open_existing(authority_root).map(|ledger| Self {
            ledger: Arc::new(ledger),
            launch_root,
        })
    }

    /// Reconstructs bounded recovery authority only for the exact current
    /// request binding and a durable reserved-or-started record.
    pub(super) fn pending_recovery(
        &self,
        context: &LiveContext,
        plan: &RoutinePlan,
        request: &RoutineEffectRequest,
    ) -> Result<Option<RoutineRecoveryAuthority>, RoutineError> {
        preflight_production_request(context, plan, request)?;
        let binding = authority_binding(request)?;
        let pending = self.ledger.pending_recovery(&binding)?;
        let Some(pending) = pending else {
            return Ok(None);
        };
        recover_staged(
            &self.launch_root,
            &pending.grant_id,
            &pending.marker,
            request.intents(),
        )?;
        Ok(Some(RoutineRecoveryAuthority {
            binding,
            grant_id: pending.grant_id,
            marker: pending.marker,
            deadline_tick: pending.deadline_tick,
        }))
    }

    pub(super) fn mediate_preflighted(
        &self,
        context: &LiveContext,
        plan: &RoutinePlan,
        request: RoutineEffectRequest,
        recovery: Option<RoutineRecoveryAuthority>,
        cancellation: RoutineCancellation,
        reuse: PreflightedProductionReuse,
        publisher: Option<&dyn RoutineArtifactPublisher>,
    ) -> Result<RoutineMediationResult, RoutineError> {
        let (reuse, reuse_claims) = reuse.into_parts();
        let authority_binding = authority_binding(&request)?;
        let recovery_for = match recovery {
            Some(recovery)
                if recovery.binding == authority_binding
                    && !recovery.grant_id.is_empty()
                    && now_tick()? <= recovery.deadline_tick =>
            {
                Some(recovery.marker)
            }
            Some(_) => return Err(error("routine-production-recovery-authority-stale")),
            None => None,
        };
        let reuse_only = recovery_for.is_none() && !reuse.is_empty();
        if !reuse_claims.is_empty() && !(reuse_only || recovery_for.is_some()) {
            return Err(error("mediator-production-reuse-not-authenticated"));
        }
        let reuse_preauthorization = if reuse_only {
            let claims = reuse_claims
                .into_iter()
                .map(|claim| ReuseArtifactClaim {
                    protocol_id: claim.protocol_id,
                    intent_id: claim.intent_id,
                    artifact_sha256: claim.artifact_sha256,
                    result_artifact_sha256: claim.result_artifact_sha256,
                    mediator_witness_sha256: claim.mediator_witness_sha256,
                })
                .collect();
            let authorization = self.ledger.preauthorize_reuse(&authority_binding, claims)?;
            Some(authorization)
        } else {
            None
        };
        let session_id = random_session_id(&authority_binding)?;
        let allowed_output_scopes = allowed_output_scopes(&request);
        let grant_binding = ProductionGrantBinding {
            session_id,
            request_id: request.request_id().to_owned(),
            protocol_id: request.protocol_id().to_owned(),
            context_id: request.context_id().to_owned(),
            candidate_id: request.candidate_id().to_owned(),
            plan_id: request.plan_id().to_owned(),
            snapshot_id: authority_binding.snapshot_id.clone(),
            allowed_output_scopes,
            recovery_for: recovery_for.clone(),
        };
        let grant_id = production_grant_identity(&grant_binding)?;
        let recovery_marker =
            production_recovery_identity(&grant_id, request.protocol_id(), request.request_id());
        let token = self.ledger.reserve(ReservationSpec {
            binding: authority_binding,
            request_id: request.request_id().to_owned(),
            grant_id,
            recovery_marker,
            recovery_for,
            reuse_only,
            reuse_preauthorization,
        })?;
        let durable = Arc::new(DurableAttempt {
            ledger: Arc::clone(&self.ledger),
            token,
            launch_root: self.launch_root.clone(),
        });
        let grant = issue_production_grant(grant_binding, durable)?;
        mediate_prepared_routine_execution(
            context,
            plan,
            PreparedRoutineExecution::Effect(request),
            Some(grant),
            cancellation,
            reuse,
            publisher,
        )
    }
}
