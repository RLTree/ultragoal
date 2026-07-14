use super::*;

/// No-op mediation stays outside authority initialization. Effectful requests
/// enter the sealed issuer and cannot supply a test grant.
pub(crate) fn mediate_prepared_routine_execution_production(
    authority_root: &Path,
    context: &LiveContext,
    plan: &RoutinePlan,
    prepared: PreparedRoutineExecution,
    recovery: Option<RoutineRecoveryAuthority>,
    cancellation: RoutineCancellation,
    reuse: RoutineReuseInput,
) -> Result<RoutineMediationResult, RoutineError> {
    if matches!(prepared, PreparedRoutineExecution::NoOp(_)) {
        if recovery.is_some() || !reuse.is_empty() {
            return Err(error("routine-production-noop-authority-or-reuse-present"));
        }
        return mediate_prepared_routine_execution(
            context,
            plan,
            prepared,
            None,
            cancellation,
            reuse,
            None,
        );
    }
    let PreparedRoutineExecution::Effect(request) = prepared else {
        unreachable!("no-op returned before production issuer selection")
    };
    // Malformed, non-canonical, and request-binding-invalid bytes reject before
    // the authority root is opened. Canonical supplied reuse then uses the
    // existing-only ledger path, which cannot initialize any durable file.
    preflight_production_request(context, plan, &request)?;
    let require_complete_reuse_set = recovery.is_none() && !reuse.is_empty();
    let reuse = preflight_production_reuse_input(reuse, &request, require_complete_reuse_set)?;
    let issuer = if reuse.is_empty() && recovery.is_none() {
        ProductionRoutineIssuer::open(authority_root)?
    } else {
        ProductionRoutineIssuer::open_existing(authority_root)?
    };
    issuer.mediate_preflighted(context, plan, request, recovery, cancellation, reuse, None)
}

pub(crate) struct DurableAttempt {
    pub(crate) ledger: Arc<FileAuthorityLedger>,
    pub(crate) token: ReservationToken,
}

impl DurableAttemptAuthority for DurableAttempt {
    fn validate_reserved(&self) -> Result<(), RoutineError> {
        self.ledger.validate_reserved(&self.token)
    }

    fn prepare_spawn(&self) -> Result<(), RoutineError> {
        self.ledger.prepare_spawn(&self.token)
    }

    fn stage_success(&self, artifacts: &BTreeMap<String, String>) -> Result<(), RoutineError> {
        self.ledger.stage_success(&self.token, artifacts)
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
pub(crate) struct EffectBinding<'a> {
    pub(crate) domain: &'static str,
    pub(crate) protocol_id: &'a str,
    pub(crate) context_id: &'a str,
    pub(crate) candidate_id: &'a str,
    pub(crate) plan_id: &'a str,
    pub(crate) result_scope: &'a str,
    pub(crate) intents: &'a [super::super::RoutineEffectIntent],
}

pub(crate) fn authority_binding(
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

pub(crate) fn allowed_output_scopes(
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

pub(crate) fn random_session_id(binding: &AuthorityBinding) -> Result<String, RoutineError> {
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

pub(crate) fn now_tick() -> Result<u64, RoutineError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|_| error("routine-production-trusted-time-unavailable"))
}

pub(crate) fn error(cause: &'static str) -> RoutineError {
    RoutineError::new(RoutineErrorId::InvalidRequest, cause, None)
}
