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

pub(super) fn owner_process_identity() -> Result<(i32, u64, u64, String), RoutineError> {
    #[cfg(target_vendor = "apple")]
    {
        let process_id = std::process::id() as i32;
        let mut info = std::mem::MaybeUninit::<libc::proc_bsdinfo>::zeroed();
        let size = std::mem::size_of::<libc::proc_bsdinfo>() as i32;
        let observed = unsafe {
            libc::proc_pidinfo(
                process_id,
                libc::PROC_PIDTBSDINFO,
                0,
                info.as_mut_ptr().cast(),
                size,
            )
        };
        if observed != size {
            return Err(error("routine-production-owner-identity-unavailable"));
        }
        let info = unsafe { info.assume_init() };
        let mut nonce = [0_u8; 32];
        getrandom::fill(&mut nonce)
            .map_err(|_| error("routine-production-authority-random-unavailable"))?;
        Ok((
            process_id,
            info.pbi_start_tvsec,
            info.pbi_start_tvusec,
            crate::routine_work::digest::sha256(&nonce),
        ))
    }
    #[cfg(not(target_vendor = "apple"))]
    {
        Err(error("routine-production-authority-host-unsupported"))
    }
}

pub(super) fn validate_observed_mediation(
    result: &RoutineMediationResult,
    settlement: DurableSettlement,
    artifacts: &[Vec<u8>],
    claimed: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, String>, RoutineError> {
    use super::super::mediator::{
        ReuseArtifactWire, RoutineMediatorStatus, RoutineNodeDisposition,
    };
    let valid_status = match settlement {
        DurableSettlement::Complete => {
            result.status() == RoutineMediatorStatus::CompleteExecution
                && result
                    .nodes()
                    .iter()
                    .all(|node| node.disposition() == RoutineNodeDisposition::Executed)
        }
        DurableSettlement::Failed => {
            result.status() == RoutineMediatorStatus::IncompleteExecution
                && result
                    .nodes()
                    .iter()
                    .any(|node| node.disposition() == RoutineNodeDisposition::Failed)
        }
        DurableSettlement::Cancelled => {
            result.status() == RoutineMediatorStatus::Cancelled
                && result
                    .nodes()
                    .iter()
                    .any(|node| node.disposition() == RoutineNodeDisposition::Cancelled)
        }
        DurableSettlement::Incomplete => {
            result.status() == RoutineMediatorStatus::IncompleteExecution
                && result.nodes().iter().all(|node| {
                    !matches!(
                        node.disposition(),
                        RoutineNodeDisposition::Failed | RoutineNodeDisposition::Cancelled
                    )
                })
        }
    };
    if !valid_status || result.recovery_required() {
        return Err(error("routine-production-mediation-outcome-invalid"));
    }
    if settlement != DurableSettlement::Complete {
        return if artifacts.is_empty() && claimed.is_empty() {
            Ok(BTreeMap::new())
        } else {
            Err(error("routine-production-terminal-artifacts-unexpected"))
        };
    }
    let expected_results = result
        .nodes()
        .iter()
        .filter_map(|node| {
            node.result_artifact_sha256()
                .map(|digest| (node.node_id(), digest))
        })
        .collect::<BTreeMap<_, _>>();
    let mut authenticated = BTreeMap::new();
    for bytes in artifacts {
        let wire: ReuseArtifactWire = serde_json::from_slice(bytes)
            .map_err(|_| error("routine-production-artifact-observation-invalid"))?;
        let digest = crate::routine_work::digest::sha256(bytes);
        let witness = super::super::mediator::reuse_witness(&wire)?;
        if crate::routine_work::digest::canonical(&wire)? != *bytes
            || wire.schema_version != "RoutineMediatedReuseArtifact-v2"
            || wire.state != "complete"
            || wire.mediator_witness_sha256 != witness
            || !super::super::mediator::result_matches_reuse(&wire)
            || crate::routine_work::digest::digest_of(&wire.result_artifact)?
                != wire.result_artifact_sha256
            || expected_results.get(wire.node_id.as_str())
                != Some(&wire.result_artifact_sha256.as_str())
            || claimed.get(&digest) != Some(&witness)
            || authenticated.insert(digest, witness).is_some()
        {
            return Err(error("routine-production-artifact-observation-invalid"));
        }
    }
    if authenticated.len() != expected_results.len() || authenticated != *claimed {
        return Err(error("routine-production-artifact-cardinality-invalid"));
    }
    Ok(authenticated)
}

pub(super) fn error(cause: &'static str) -> RoutineError {
    RoutineError::new(RoutineErrorId::InvalidRequest, cause, None)
}
