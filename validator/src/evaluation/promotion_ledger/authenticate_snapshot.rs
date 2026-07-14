fn authenticate_snapshot(
    payload: ReviewSnapshotPayload,
    key: &[u8; 32],
) -> Result<AuthenticatedReviewSnapshot, PromotionLedgerError> {
    let bytes = serde_json::to_vec(&payload)
        .map_err(|_| PromotionLedgerError::new("promotion-ledger-serialization-failed"))?;
    let mac_sha256 = hmac(&bytes, key).map_err(map_storage)?;
    let head_sha256 = sha256(format!("{}|{mac_sha256}", sha256(&bytes)).as_bytes());
    Ok(AuthenticatedReviewSnapshot {
        payload,
        mac_sha256,
        head_sha256,
    })
}

fn authenticate_anchor_record(
    payload: ReviewAnchorRecordPayload,
    key: &[u8; 32],
) -> Result<AuthenticatedReviewAnchorRecord, PromotionLedgerError> {
    let bytes = serde_json::to_vec(&payload)
        .map_err(|_| PromotionLedgerError::new("promotion-ledger-serialization-failed"))?;
    let mac_sha256 = hmac(&bytes, key).map_err(map_storage)?;
    let head_sha256 = sha256(format!("{}|{mac_sha256}", sha256(&bytes)).as_bytes());
    Ok(AuthenticatedReviewAnchorRecord {
        payload,
        mac_sha256,
        head_sha256,
    })
}

fn read_current(
    root: &File,
    anchor: &File,
    lock_identity: FileIdentity,
    anchor_authority: FileAuthorityIdentity,
    key: &[u8; 32],
    key_id: &str,
    binding: &PromotionLedgerBinding,
) -> Result<CurrentReviewSnapshot, PromotionLedgerError> {
    require_named_review_lock_identity(root, lock_identity)?;
    require_named_review_anchor_authority(root, anchor_authority)?;
    let snapshot: AuthenticatedReviewSnapshot =
        read_json_file(root, STATE_NAME).map_err(map_storage)?;
    let expected = authenticate_snapshot(snapshot.payload.clone(), key)?;
    if snapshot.payload.core.schema_version != "PromotionReviewLedger-v1"
        || snapshot.payload.core.key_id != key_id
        || snapshot.payload.core.lock_identity != lock_identity
        || snapshot.payload.core.anchor_authority != anchor_authority
        || snapshot.payload.core.binding != *binding
        || snapshot.payload.anchor_observation.authority() != anchor_authority
        || snapshot.payload.anchor_length != snapshot.payload.anchor_observation.length
        || !super::valid_sha256(&snapshot.payload.anchor_head_sha256)
        || snapshot.mac_sha256 != expected.mac_sha256
        || snapshot.head_sha256 != expected.head_sha256
        || !state_valid(&snapshot.payload.core.state)
    {
        return Err(PromotionLedgerError::new(
            "promotion-ledger-authentication-failed",
        ));
    }
    let observed_state_head_sha256 = snapshot.head_sha256.clone();
    let observation_before = safe_file_identity(anchor).map_err(map_storage)?;
    if observation_before.authority() != anchor_authority {
        return Err(PromotionLedgerError::new(
            "promotion-anchor-descriptor-substituted",
        ));
    }
    let bytes = read_anchor_bytes(anchor)?;
    let observation_after = safe_file_identity(anchor).map_err(map_storage)?;
    if observation_after != observation_before {
        return Err(PromotionLedgerError::new(
            "promotion-anchor-changed-during-read",
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
        return Err(PromotionLedgerError::new(
            "promotion-anchor-state-binding-invalid",
        ));
    };
    let extra = &scan.records[stored_index + 1..];
    match extra {
        [] => {
            if scan.complete_length != snapshot.payload.anchor_length {
                return Err(PromotionLedgerError::new(
                    "promotion-anchor-journal-ambiguous",
                ));
            }
            if scan.partial_tail {
                if observation_after.length <= scan.complete_length {
                    return Err(PromotionLedgerError::new(
                        "promotion-anchor-partial-tail-invalid",
                    ));
                }
                Ok(CurrentReviewSnapshot {
                    snapshot,
                    partial_tail_from: Some(scan.complete_length),
                    observed_state_head_sha256,
                    observed_anchor: observation_after,
                })
            } else if observation_after == snapshot.payload.anchor_observation
                && observation_after.length == snapshot.payload.anchor_length
            {
                Ok(CurrentReviewSnapshot {
                    snapshot,
                    partial_tail_from: None,
                    observed_state_head_sha256,
                    observed_anchor: observation_after,
                })
            } else {
                Err(PromotionLedgerError::new(
                    "promotion-anchor-rollback-or-mutation-detected",
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
                return Err(PromotionLedgerError::new(
                    "promotion-anchor-recovery-chain-invalid",
                ));
            }
            let recovered = authenticate_snapshot(
                ReviewSnapshotPayload {
                    core: successor.record.payload.core.clone(),
                    anchor_observation: observation_after,
                    anchor_length: successor.end,
                    anchor_head_sha256: successor.record.head_sha256.clone(),
                },
                key,
            )?;
            Ok(CurrentReviewSnapshot {
                snapshot: recovered,
                partial_tail_from: None,
                observed_state_head_sha256,
                observed_anchor: observation_after,
            })
        }
        _ => Err(PromotionLedgerError::new(
            "promotion-anchor-recovery-depth-exceeded",
        )),
    }
}

fn verify_anchor_record(
    record: &AuthenticatedReviewAnchorRecord,
    key: &[u8; 32],
) -> Result<(), PromotionLedgerError> {
    let expected = authenticate_anchor_record(record.payload.clone(), key)?;
    if record.mac_sha256 != expected.mac_sha256 || record.head_sha256 != expected.head_sha256 {
        return Err(PromotionLedgerError::new(
            "promotion-anchor-authentication-failed",
        ));
    }
    Ok(())
}

struct ScannedReviewAnchorRecord {
    end: u64,
    record: AuthenticatedReviewAnchorRecord,
}

struct ReviewAnchorJournalScan {
    records: Vec<ScannedReviewAnchorRecord>,
    complete_length: u64,
    partial_tail: bool,
}
