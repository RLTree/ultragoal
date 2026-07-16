use super::*;

pub(super) fn reservation_spec(
    request: &RoutineEffectRequest,
    binding: AuthorityBinding,
    output_journal: OutputProvisionJournal,
) -> Result<ReservationSpec, RoutineError> {
    let scopes = allowed_output_scopes(request);
    let session_id = random_session_id(&binding)?;
    let grant_id = reservation_grant_id(&session_id, request, &binding, &scopes)?;
    Ok(ReservationSpec {
        binding,
        request_id: request.request_id().to_owned(),
        recovery_marker: recovery_identity(&grant_id, request.protocol_id(), request.request_id()),
        grant_id,
        output_journal,
        owner: owner_lease()?,
        intents: request
            .intents()
            .iter()
            .map(request_intent_binding)
            .collect(),
    })
}

pub(super) fn launch_stage_record(
    staged: &super::super::super::super::mediator::StagedProgram,
    intent: &super::super::super::super::mediator::IntentExecutionRequest,
) -> store::LaunchStageRecord {
    store::LaunchStageRecord {
        directory: launch_identity(staged.directory_identity),
        program: launch_identity(staged.executable.identity),
        marker: launch_identity(staged.marker_identity),
        seal: launch_identity(staged.seal_identity),
        program_sha256: staged.executable.sha256.clone(),
        intent: observed_intent_binding(intent, &staged.executable.sha256),
    }
}

pub(super) fn observed_intent_binding(
    intent: &super::super::super::super::mediator::IntentExecutionRequest,
    program_sha256: &str,
) -> store::IntentBinding {
    store::IntentBinding {
        intent_id: intent.intent_id().to_owned(),
        node_id: intent.node_id().to_owned(),
        plan_order: intent.plan_order(),
        program_sha256: program_sha256.to_owned(),
    }
}

fn request_intent_binding(
    intent: &super::super::super::super::RoutineEffectIntent,
) -> store::IntentBinding {
    store::IntentBinding {
        intent_id: intent.intent_id().to_owned(),
        node_id: intent.node_id().to_owned(),
        plan_order: intent.plan_order(),
        program_sha256: intent.program_sha256().to_owned(),
    }
}

fn launch_identity(
    identity: super::super::super::super::mediator::ObjectIdentity,
) -> store::LaunchEntryIdentity {
    store::LaunchEntryIdentity {
        device: identity.device,
        inode: identity.inode,
        mode: identity.mode,
        owner: identity.owner_user_id,
        links: identity.links,
        length: identity.length,
    }
}

fn owner_lease() -> Result<OwnerLease, RoutineError> {
    let (process_id, start_seconds, start_microseconds, nonce_sha256) = owner_process_identity()?;
    Ok(OwnerLease {
        process_id,
        start_seconds,
        start_microseconds,
        nonce_sha256,
    })
}
