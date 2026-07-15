use super::launch_custody::cleanup_staged;
use super::launch_snapshot::stage_program;
use super::*;
use crate::routine_work::runtime_adapter::mediator::{PinnedExecutable, StagedProgram};

/// Internal durable machinery. The production parent is the only module that
/// may compose these helpers into the canonical public adapter.

pub(super) struct DurableAttempt {
    pub(crate) ledger: Arc<FileAuthorityLedger>,
    pub(crate) token: ReservationToken,
    pub(crate) launch_root: PathBuf,
}

impl DurableAttemptAuthority for DurableAttempt {
    fn validate_reserved(&self) -> Result<(), RoutineError> {
        self.ledger.validate_reserved(&self.token)
    }

    fn stage_program(&self, program: &PinnedExecutable) -> Result<StagedProgram, RoutineError> {
        stage_program(&self.launch_root, &self.token, program)
    }

    fn cleanup_staged(&self, staged: &StagedProgram) -> Result<(), RoutineError> {
        cleanup_staged(staged)
    }

    fn prepare_spawn(&self) -> Result<(), RoutineError> {
        self.ledger.prepare_spawn(&self.token)
    }

    fn stage_success(&self, artifacts: &BTreeMap<String, String>) -> Result<(), RoutineError> {
        self.ledger.stage_success(&self.token, artifacts)
    }

    fn record_failure(&self, evidence: &ReservationFailureEvidence) -> Result<(), RoutineError> {
        self.ledger.record_failure(&self.token, evidence)
    }

    fn settle(
        &self,
        outcome: DurableSettlement,
        artifacts: &BTreeMap<String, String>,
    ) -> Result<(), RoutineError> {
        let state = match outcome {
            DurableSettlement::Complete => AttemptState::Complete,
            DurableSettlement::Failed => AttemptState::Failed,
            DurableSettlement::Cancelled => AttemptState::Cancelled,
            DurableSettlement::Incomplete => AttemptState::Incomplete,
        };
        self.ledger.settle(&self.token, state, artifacts)
    }

    fn authenticates_artifact(&self, digest: &str, witness: &str) -> Result<bool, RoutineError> {
        self.ledger.authenticates(&self.token, digest, witness)
    }

    fn recovery_is_durable(&self) -> bool {
        self.token.recovery_for.is_some()
    }

    fn reuse_only(&self) -> bool {
        self.token.reuse_only
    }
}

#[derive(Serialize)]
pub(super) struct EffectBinding<'a> {
    pub(crate) domain: &'static str,
    pub(crate) protocol_id: &'a str,
    pub(crate) context_id: &'a str,
    pub(crate) candidate_id: &'a str,
    pub(crate) plan_id: &'a str,
    pub(crate) result_scope: &'a str,
    pub(crate) intents: &'a [super::super::RoutineEffectIntent],
}

pub(super) fn authority_binding(
    request: &RoutineEffectRequest,
) -> Result<AuthorityBinding, RoutineError> {
    let effect_id = digest_of(&EffectBinding {
        domain: "routine-production-semantic-effect-v1",
        protocol_id: request.protocol_id(),
        context_id: request.context_id(),
        candidate_id: request.candidate_id(),
        plan_id: request.plan_id(),
        result_scope: request.result_scope(),
        intents: request.intents(),
    })?;
    Ok(AuthorityBinding {
        protocol_id: request.protocol_id().to_owned(),
        effect_id,
        context_id: request.context_id().to_owned(),
        candidate_id: request.candidate_id().to_owned(),
        plan_id: request.plan_id().to_owned(),
        snapshot_id: request.snapshot_id.clone(),
    })
}

pub(super) fn allowed_output_scopes(
    request: &RoutineEffectRequest,
) -> Vec<crate::routine_work::RepoPath> {
    let mut scopes = request
        .intents()
        .iter()
        .flat_map(|intent| intent.declared_output_scopes().iter().cloned())
        .collect::<Vec<_>>();
    scopes.sort_by(|left, right| left.as_str().cmp(right.as_str()));
    scopes.dedup();
    scopes
}

pub(super) fn random_session_id(binding: &AuthorityBinding) -> Result<String, RoutineError> {
    let mut nonce = [0u8; 32];
    getrandom::fill(&mut nonce)
        .map_err(|_| error("routine-production-authority-random-unavailable"))?;
    Ok(framed(&[
        b"routine-production-session-v1",
        &nonce,
        binding.protocol_id.as_bytes(),
        binding.effect_id.as_bytes(),
    ]))
}

pub(super) fn now_tick() -> Result<u64, RoutineError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|_| error("routine-production-trusted-time-unavailable"))
}

pub(super) fn error(cause: &'static str) -> RoutineError {
    RoutineError::new(RoutineErrorId::InvalidRequest, cause, None)
}
