#[cfg(test)]
pub(super) fn propose_recovery(
    classification: &PublicationClassification,
    authorization: RecoveryAuthorization,
    coordinator_binding_sha256: &str,
    head: &HostEffectLedgerHead,
    trusted_now_unix_ms: u64,
) -> Result<RecoveryProposal, SupportedHostLifecycleError> {
    let expected = authorization_digest(
        &authorization.schema_version,
        &authorization.classification_sha256,
        &authorization.coordinator_binding_sha256,
        &authorization.ledger_head,
        authorization.issued_at_unix_ms,
        authorization.expires_at_unix_ms,
        &authorization.nonce_sha256,
    )?;
    if expected != authorization.authorization_sha256
        || authorization.schema_version != RECOVERY_AUTHORIZATION_SCHEMA
        || authorization.classification_sha256 != classification.evidence_sha256
        || authorization.coordinator_binding_sha256 != coordinator_binding_sha256
        || &authorization.ledger_head != head
        || trusted_now_unix_ms < authorization.issued_at_unix_ms
        || trusted_now_unix_ms > authorization.expires_at_unix_ms
    {
        return Err(recovery_authorization_required());
    }
    let action = match classification.id {
        PublicationClassificationId::CleanPriorState => {
            RecoveryProposalAction::ResumeWithoutMutation
        }
        PublicationClassificationId::InterruptedBeforeTempWrite
        | PublicationClassificationId::InterruptedDuringTempFsync
        | PublicationClassificationId::InterruptedBeforeRename
        | PublicationClassificationId::OrphanedTemporaryObject => {
            RecoveryProposalAction::QuarantineTemporaryObjectForReview
        }
        PublicationClassificationId::CommittedBeforeAcknowledgement => {
            RecoveryProposalAction::ReconcileCommittedStateBeforeAcknowledgement
        }
        _ => {
            return Err(lifecycle_error(
                SupportedHostLifecycleErrorId::RecoveryUnsafe,
            ));
        }
    };
    #[derive(Serialize)]
    struct Proposal<'a> {
        schema: &'static str,
        action: RecoveryProposalAction,
        classification_sha256: &'a str,
        authorization_sha256: &'a str,
        authorized_ledger_head: &'a HostEffectLedgerHead,
        automatic_cleanup: bool,
    }
    let proposal_sha256 = digest_json(&Proposal {
        schema: "harness-ultragoal.host-effect-recovery-proposal.v2",
        action,
        classification_sha256: &classification.evidence_sha256,
        authorization_sha256: &authorization.authorization_sha256,
        authorized_ledger_head: &authorization.ledger_head,
        automatic_cleanup: false,
    })?;
    Ok(RecoveryProposal {
        action,
        classification_sha256: classification.evidence_sha256.clone(),
        proposal_sha256,
        automatic_cleanup: false,
    })
}

fn classify_publication(
    observation: &PublicationInventoryObservation,
) -> Result<PublicationClassification, SupportedHostLifecycleError> {
    let id = if observation.scan_generation_before != observation.scan_generation_after
        || observation.target.object_generation != observation.scan_generation_before
        || observation
            .temporary_objects
            .iter()
            .any(|object| object.object_generation != observation.scan_generation_before)
    {
        PublicationClassificationId::ObservationRace
    } else if !matches!(
        observation.target.kind,
        PublicationObjectKind::Missing | PublicationObjectKind::Regular
    ) || observation.temporary_objects.iter().any(|object| {
        object.kind != PublicationObjectKind::Regular
            || object.mode & 0o170000 != 0o100000
            || object.hard_links != 1
    }) {
        PublicationClassificationId::UnsafeSpecialObject
    } else if observation.temporary_objects.len() > 1 {
        PublicationClassificationId::MultipleTemporaryObjects
    } else if let Some(acknowledgement) = &observation.acknowledgement {
        if exact_next_target(observation)
            && observation.temporary_objects.is_empty()
            && acknowledgement
                .matches(&observation.expectation, &observation.current_ledger_head)?
        {
            PublicationClassificationId::AcknowledgedCommitted
        } else {
            PublicationClassificationId::FalsePassReceipt
        }
    } else if let Some(temporary) = observation.temporary_objects.first() {
        if exact_next_target(observation) {
            PublicationClassificationId::OrphanedTemporaryObject
        } else if !prior_target_matches(observation)
            || !observation
                .expectation
                .temporary
                .safe_temporary_metadata_matches(temporary)
        {
            PublicationClassificationId::OrphanedTemporaryObject
        } else if temporary.byte_length == 0
            && temporary.content_sha256.is_none()
            && !temporary.data_synced
        {
            PublicationClassificationId::InterruptedBeforeTempWrite
        } else if !temporary.data_synced {
            PublicationClassificationId::InterruptedDuringTempFsync
        } else if observation.expectation.temporary.matches(temporary) {
            PublicationClassificationId::InterruptedBeforeRename
        } else {
            PublicationClassificationId::OrphanedTemporaryObject
        }
    } else if exact_next_target(observation) {
        PublicationClassificationId::CommittedBeforeAcknowledgement
    } else if prior_target_matches(observation) {
        PublicationClassificationId::CleanPriorState
    } else {
        PublicationClassificationId::UnknownState
    };
    #[derive(Serialize)]
    struct Classification<'a> {
        schema: &'static str,
        id: PublicationClassificationId,
        observation: &'a PublicationInventoryObservation,
    }
    Ok(PublicationClassification {
        id,
        evidence_sha256: digest_json(&Classification {
            schema: "harness-ultragoal.prepublication-recovery-classification.v2",
            id,
            observation,
        })?,
        temporary_object: observation
            .temporary_objects
            .first()
            .map(|object| object.name.clone()),
    })
}

fn exact_next_target(observation: &PublicationInventoryObservation) -> bool {
    observation.expectation.next.matches(&observation.target)
}

fn prior_target_matches(observation: &PublicationInventoryObservation) -> bool {
    observation.expectation.prior.matches(&observation.target)
}

fn temporary_name(value: &str, target: &str) -> bool {
    let Some(rest) = value.strip_prefix(&format!(".{target}.")) else {
        return false;
    };
    let Some(hex) = rest.strip_suffix(".tmp") else {
        return false;
    };
    hex.len() == 16
        && hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn valid_object_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 255
        && !matches!(value, "." | "..")
        && !value
            .bytes()
            .any(|byte| byte == b'/' || byte == 0 || byte.is_ascii_control())
}

fn is_digest(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}
