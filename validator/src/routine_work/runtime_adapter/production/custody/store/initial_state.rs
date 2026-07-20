use super::store_open::current_user_id;
use super::*;

pub(super) fn initial_payload(
    authority_id: &str,
    key_id: &str,
    root_identity: RootIdentity,
    lock_identity: FileIdentity,
) -> Result<Payload, RoutineError> {
    Ok(Payload {
        schema_version: SCHEMA.to_owned(),
        authority_id: authority_id.to_owned(),
        key_id: key_id.to_owned(),
        root_identity,
        lock_identity,
        generation: 0,
        previous_head_sha256: sha256(&canonical(&(
            "routine-production-authority-initial-head-v1",
            authority_id,
        ))?),
        last_tick: now_tick()?,
        attempts: BTreeMap::new(),
        effects: BTreeMap::new(),
        consumed_grants: BTreeSet::new(),
    })
}

pub(super) fn authority_id(
    key_id: &str,
    root: RootIdentity,
    lock: FileIdentity,
) -> Result<String, RoutineError> {
    canonical(&("routine-production-authority-v1", key_id, root, lock)).map(|bytes| sha256(&bytes))
}

pub(super) fn encode(payload: &Payload, key: &LedgerKey) -> Result<Vec<u8>, RoutineError> {
    let payload_bytes = canonical(payload)?;
    let envelope = Envelope {
        payload: payload.clone(),
        hmac_sha256: hmac(key.bytes(), &payload_bytes)?,
    };
    canonical(&envelope)
}

pub(super) fn decode(
    bytes: &[u8],
    key: &LedgerKey,
    authority_id: &str,
    key_id: &str,
    root_identity: RootIdentity,
    lock_identity: FileIdentity,
) -> Result<Payload, RoutineError> {
    let envelope: Envelope = serde_json::from_slice(bytes)
        .map_err(|_| error("routine-production-authority-state-invalid"))?;
    if canonical(&envelope)? != bytes
        || envelope.payload.schema_version != SCHEMA
        || envelope.payload.authority_id != authority_id
        || envelope.payload.key_id != key_id
        || envelope.payload.root_identity != root_identity
        || envelope.payload.lock_identity != lock_identity
        || envelope.hmac_sha256 != hmac(key.bytes(), &canonical(&envelope.payload)?)?
    {
        return Err(error("routine-production-authority-state-tampered"));
    }
    Ok(envelope.payload)
}

pub(super) fn validate_payload(payload: &Payload) -> Result<(), RoutineError> {
    if !valid(&payload.authority_id)
        || !valid(&payload.key_id)
        || !valid(&payload.previous_head_sha256)
        || payload.attempts.len() > MAX_RECORDS
        || payload.effects.len() > MAX_RECORDS
        || payload.consumed_grants.len() > MAX_RECORDS.saturating_mul(4)
        || payload.attempts.iter().any(|(grant, record)| {
            grant != &record.grant_id
                || validate_binding(&record.binding).is_err()
                || !valid(&record.request_id)
                || !valid(&record.grant_id)
                || !valid(&record.recovery_marker)
                || record.issued_tick > record.expires_tick
                || record.expires_tick > record.recovery_deadline_tick
                || !valid_owner(&record.owner)
                || record
                    .child
                    .as_ref()
                    .is_some_and(|child| !valid_child(child))
                || record.intents.is_empty()
                || record.next_intent > record.intents.len()
                || record.intents.iter().any(|intent| !valid_intent(intent))
                || record
                    .launch_stage
                    .as_ref()
                    .is_some_and(|stage| !valid_launch_stage(stage))
                || record.failure_evidence.len() > 32
                || record.failure_evidence.iter().any(|evidence| {
                    !evidence.shape_is_valid() || evidence.protocol_id != record.binding.protocol_id
                })
                || record
                    .terminal
                    .as_ref()
                    .is_some_and(|terminal| !valid_terminal(terminal))
                || record
                    .publication_ambiguity
                    .as_ref()
                    .is_some_and(|ambiguity| !valid_publication_ambiguity(ambiguity, record))
                || !valid_terminal_binding(record)
                || validate_output_journal(&record.output_journal).is_err()
        })
        || payload.effects.iter().any(|(effect, protocol)| {
            !valid(effect)
                || !valid(protocol)
                || !payload.attempts.values().any(|record| {
                    &record.binding.protocol_id == protocol && &record.binding.effect_id == effect
                })
        })
        || payload.consumed_grants.iter().any(|grant| !valid(grant))
    {
        return Err(error("routine-production-authority-state-shape-invalid"));
    }
    Ok(())
}

fn valid_owner(owner: &OwnerLease) -> bool {
    owner.process_id > 0
        && (owner.start_seconds != 0 || owner.start_microseconds != 0)
        && valid(&owner.nonce_sha256)
}

fn valid_child(child: &ChildLease) -> bool {
    child.process_id > 0
        && child.process_group_id > 0
        && valid(&child.executable_sha256)
        && child.executable_device != 0
        && child.executable_inode != 0
        && valid_intent(&child.intent)
}

fn valid_intent(intent: &IntentBinding) -> bool {
    valid(&intent.intent_id) && !intent.node_id.is_empty() && valid(&intent.program_sha256)
}

fn valid_launch_stage(stage: &LaunchStageRecord) -> bool {
    let regular = |entry: LaunchEntryIdentity| {
        entry.device != 0
            && entry.inode != 0
            && entry.owner == current_user_id()
            && entry.mode & u32::from(libc::S_IFMT) == u32::from(libc::S_IFREG)
            && entry.links == 1
    };
    stage.directory.device != 0
        && stage.directory.inode != 0
        && stage.directory.owner == current_user_id()
        && stage.directory.mode & u32::from(libc::S_IFMT) == u32::from(libc::S_IFDIR)
        && valid(&stage.program_sha256)
        && valid_intent(&stage.intent)
        && [stage.program, stage.marker, stage.seal]
            .into_iter()
            .all(regular)
}

fn valid_terminal(terminal: &TerminalRecord) -> bool {
    !terminal.state.pending()
        && terminal.state != AttemptState::Ambiguous
        && valid(&terminal.result_sha256)
        && valid(&terminal.prior_head_sha256)
        && terminal
            .artifacts
            .iter()
            .all(|(digest, witness)| valid(digest) && valid(witness))
        && terminal.process_cleanup.shape_is_valid()
        && terminal.staged_cleanup.shape_is_valid()
        && terminal
            .failure_evidence
            .as_ref()
            .is_none_or(ReservationFailureEvidence::shape_is_valid)
        && (terminal.state == AttemptState::Complete) == terminal.mediation.is_some()
        && terminal.mediation.as_ref().is_none_or(|mediation| {
            !mediation.nodes.is_empty()
                && mediation.nodes.iter().all(|node| {
                    !node.intent_id.is_empty()
                        && !node.node_id.is_empty()
                        && valid(&node.result_artifact_sha256)
                })
        })
}

fn valid_publication_ambiguity(ambiguity: &PublicationAmbiguity, record: &ProtocolRecord) -> bool {
    valid(&ambiguity.previous_head_sha256)
        && valid(&ambiguity.proposed_head_sha256)
        && !ambiguity.cause.is_empty()
        && ambiguity.failure_evidence.as_ref().is_none_or(|evidence| {
            evidence.shape_is_valid()
                && evidence.protocol_id == record.binding.protocol_id
                && evidence.grant_id == record.grant_id
                && evidence.recovery_marker == record.recovery_marker
        })
}

fn valid_terminal_binding(record: &ProtocolRecord) -> bool {
    match record.state {
        AttemptState::Reserved | AttemptState::Staged | AttemptState::Started => {
            record.terminal.is_none() && record.publication_ambiguity.is_none()
        }
        AttemptState::Complete
        | AttemptState::Failed
        | AttemptState::Cancelled
        | AttemptState::Incomplete => {
            record.child.is_none()
                && record.launch_stage.is_none()
                && record.terminal.as_ref().is_some_and(|terminal| {
                    terminal.state == record.state
                        && (record.state != AttemptState::Complete || terminal.mediation.is_some())
                        && terminal.mediation.as_ref().is_none_or(|mediation| {
                            mediation.nodes.len() == record.intents.len()
                                && mediation.nodes.iter().zip(&record.intents).all(
                                    |(node, intent)| {
                                        node.intent_id == intent.intent_id
                                            && node.node_id == intent.node_id
                                            && node.plan_order == intent.plan_order
                                    },
                                )
                        })
                        && terminal.failure_evidence.as_ref().is_none_or(|evidence| {
                            evidence.protocol_id == record.binding.protocol_id
                                && evidence.grant_id == record.grant_id
                                && evidence.recovery_marker == record.recovery_marker
                        })
                })
        }
        AttemptState::RolledBack => record.publication_ambiguity.is_none(),
        AttemptState::Ambiguous => record.publication_ambiguity.is_some(),
    }
}

pub(super) fn validate_token(token: &ReservationToken) -> Result<(), RoutineError> {
    validate_binding(&token.binding)?;
    if !valid(&token.request_id)
        || !valid(&token.grant_id)
        || !valid(&token.recovery_marker)
        || !valid_owner(&token.owner)
        || token.intents.is_empty()
        || token.intents.iter().any(|intent| !valid_intent(intent))
        || validate_output_journal(&token.output_journal).is_err()
    {
        return Err(error("routine-production-reservation-spec-invalid"));
    }
    Ok(())
}

pub(super) fn validate_binding(binding: &AuthorityBinding) -> Result<(), RoutineError> {
    if [
        &binding.protocol_id,
        &binding.effect_id,
        &binding.context_id,
        &binding.candidate_id,
        &binding.plan_id,
        &binding.snapshot_id,
    ]
    .into_iter()
    .all(|value| valid(value))
    {
        Ok(())
    } else {
        Err(error("routine-production-binding-invalid"))
    }
}
