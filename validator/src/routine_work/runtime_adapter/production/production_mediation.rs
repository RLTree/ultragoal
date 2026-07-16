use super::*;

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

pub(super) fn reservation_grant_id(
    session_id: &str,
    request: &RoutineEffectRequest,
    binding: &AuthorityBinding,
    scopes: &[crate::routine_work::RepoPath],
    recovery_for: Option<&str>,
) -> Result<String, RoutineError> {
    digest_of(&(
        "routine-production-reservation-v1",
        session_id,
        request.request_id(),
        request.protocol_id(),
        request.context_id(),
        request.candidate_id(),
        request.plan_id(),
        &binding.snapshot_id,
        scopes,
        recovery_for,
    ))
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
