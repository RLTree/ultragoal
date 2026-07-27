fn read_current(
    root: &File,
    anchor: &File,
    lock_identity: FileIdentity,
    anchor_authority: FileAuthorityIdentity,
    key: &[u8; 32],
    key_id: &str,
    binding: &EvaluationExecutionBinding,
) -> Result<CurrentSnapshot, EvaluationLedgerError> {
    require_named_lock_identity(root, lock_identity)?;
    require_named_anchor_authority(root, anchor_authority)?;
    let snapshot: AuthenticatedSnapshot = read_json_file(root, STATE_NAME)?;
    verify_snapshot(
        &snapshot,
        key,
        key_id,
        lock_identity,
        anchor_authority,
        binding,
    )?;
    let observed_state_head_sha256 = snapshot.head_sha256.clone();
    let observation_before = safe_file_identity(anchor)?;
    if observation_before.authority() != anchor_authority {
        return Err(EvaluationLedgerError::new(
            "evaluation-anchor-descriptor-substituted",
        ));
    }
    let bytes = read_file_bytes(anchor, MAX_ANCHOR_JOURNAL_BYTES)?;
    let observation_after = safe_file_identity(anchor)?;
    if observation_after != observation_before {
        return Err(EvaluationLedgerError::new(
            "evaluation-anchor-changed-during-read",
        ));
    }
    let scan = scan_anchor_journal(
        &bytes,
        key,
        key_id,
        lock_identity,
        anchor_authority,
        binding,
    )?;
    let stored_index = scan.records.iter().position(|record| {
        record.end == snapshot.payload.anchor_length
            && record.record.head_sha256 == snapshot.payload.anchor_head_sha256
            && record.record.payload.core == snapshot.payload.core
    });
    let Some(stored_index) = stored_index else {
        return Err(EvaluationLedgerError::new(
            "evaluation-anchor-state-binding-invalid",
        ));
    };
    let extra = &scan.records[stored_index + 1..];
    match extra {
        [] => {
            if scan.complete_length != snapshot.payload.anchor_length {
                return Err(EvaluationLedgerError::new(
                    "evaluation-anchor-journal-ambiguous",
                ));
            }
            if scan.partial_tail {
                if observation_after.length <= scan.complete_length {
                    return Err(EvaluationLedgerError::new(
                        "evaluation-anchor-partial-tail-invalid",
                    ));
                }
                Ok(CurrentSnapshot {
                    snapshot,
                    partial_tail_from: Some(scan.complete_length),
                    observed_state_head_sha256,
                    observed_anchor: observation_after,
                })
            } else if observation_after == snapshot.payload.anchor_observation
                && observation_after.length == snapshot.payload.anchor_length
            {
                Ok(CurrentSnapshot {
                    snapshot,
                    partial_tail_from: None,
                    observed_state_head_sha256,
                    observed_anchor: observation_after,
                })
            } else {
                Err(EvaluationLedgerError::new(
                    "evaluation-anchor-rollback-or-mutation-detected",
                ))
            }
        }
        [successor] => {
            if scan.partial_tail
                || successor.record.payload.core.generation
                    != snapshot.payload.core.generation.saturating_add(1)
                || successor.record.payload.core.previous_head_sha256 != snapshot.head_sha256
                || successor.record.payload.prior_anchor_head_sha256
                    != snapshot.payload.anchor_head_sha256
            {
                return Err(EvaluationLedgerError::new(
                    "evaluation-anchor-recovery-chain-invalid",
                ));
            }
            let recovered = authenticate_snapshot(
                SnapshotPayload {
                    core: successor.record.payload.core.clone(),
                    anchor_observation: observation_after,
                    anchor_length: successor.end,
                    anchor_head_sha256: successor.record.head_sha256.clone(),
                },
                key,
            )?;
            Ok(CurrentSnapshot {
                snapshot: recovered,
                partial_tail_from: None,
                observed_state_head_sha256,
                observed_anchor: observation_after,
            })
        }
        _ => Err(EvaluationLedgerError::new(
            "evaluation-anchor-recovery-depth-exceeded",
        )),
    }
}

fn authenticate_snapshot(
    payload: SnapshotPayload,
    key: &[u8; 32],
) -> Result<AuthenticatedSnapshot, EvaluationLedgerError> {
    let payload_bytes = serde_json::to_vec(&payload)
        .map_err(|_| EvaluationLedgerError::new("evaluation-ledger-serialization-failed"))?;
    let mac_sha256 = hmac(&payload_bytes, key)?;
    let head_sha256 = sha256(format!("{}|{mac_sha256}", sha256(&payload_bytes)).as_bytes());
    Ok(AuthenticatedSnapshot {
        payload,
        mac_sha256,
        head_sha256,
    })
}

fn verify_snapshot(
    snapshot: &AuthenticatedSnapshot,
    key: &[u8; 32],
    key_id: &str,
    lock_identity: FileIdentity,
    anchor_authority: FileAuthorityIdentity,
    binding: &EvaluationExecutionBinding,
) -> Result<(), EvaluationLedgerError> {
    let expected = authenticate_snapshot(snapshot.payload.clone(), key)?;
    if snapshot.payload.core.schema_version != "EvaluationExecutionLedger-v1"
        || snapshot.payload.core.key_id != key_id
        || snapshot.payload.core.lock_identity != lock_identity
        || snapshot.payload.core.anchor_authority != anchor_authority
        || snapshot.payload.core.binding != *binding
        || !snapshot_reservation_identity_valid(
            &snapshot.payload.core.state,
            snapshot.payload.core.reservation_id_sha256.as_deref(),
        )
        || snapshot.payload.anchor_observation.authority() != anchor_authority
        || snapshot.payload.anchor_length != snapshot.payload.anchor_observation.length
        || !super::valid_sha256(&snapshot.payload.anchor_head_sha256)
        || snapshot.mac_sha256 != expected.mac_sha256
        || snapshot.head_sha256 != expected.head_sha256
        || !state_valid(&snapshot.payload.core.state)
    {
        return Err(EvaluationLedgerError::new(
            "evaluation-ledger-authentication-failed",
        ));
    }
    Ok(())
}

fn snapshot_reservation_identity_valid(state: &EvaluationLedgerState, value: Option<&str>) -> bool {
    match state {
        EvaluationLedgerState::Initialized => value.is_none(),
        _ => value.is_some_and(super::valid_sha256),
    }
}

fn authenticate_anchor_record(
    payload: AnchorRecordPayload,
    key: &[u8; 32],
) -> Result<AuthenticatedAnchorRecord, EvaluationLedgerError> {
    let bytes = serde_json::to_vec(&payload)
        .map_err(|_| EvaluationLedgerError::new("evaluation-ledger-serialization-failed"))?;
    let mac_sha256 = hmac(&bytes, key)?;
    let head_sha256 = sha256(format!("{}|{mac_sha256}", sha256(&bytes)).as_bytes());
    Ok(AuthenticatedAnchorRecord {
        payload,
        mac_sha256,
        head_sha256,
    })
}
