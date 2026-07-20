impl DurableHostEffectLedger for FileHostEffectLedger {
    fn head(&self) -> Result<HostEffectLedgerHead, HostEffectLedgerError> {
        self.with_snapshot(|payload, _, _| {
            Ok((
                HostEffectLedgerHead::new(payload.generation, payload.head_sha256.clone())?,
                false,
            ))
        })
    }

    fn reserve(
        &self,
        reservation: HostEffectReservation,
    ) -> Result<HostEffectLedgerRecord, HostEffectLedgerError> {
        self.with_snapshot(|payload, replayed, _| {
            validate_reservation(&reservation, &self.ledger_id)?;
            if reservation.expected_head_sha256 != payload.head_sha256 {
                return Err(stale_head());
            }
            if replayed.records.contains_key(&reservation.permit_id)
                || replayed.nonces.contains(&reservation.nonce_sha256)
                || replayed
                    .semantic_keys
                    .contains(&reservation.semantic_key_sha256)
            {
                return Err(replay_error());
            }
            let event = reserve_event(payload, &reservation)?;
            payload.events.push(event);
            payload.generation += 1;
            payload.head_sha256 = payload
                .events
                .last()
                .ok_or_else(invalid_record)?
                .current_head_sha256
                .clone();
            let next = replay(payload)?;
            let record = next
                .records
                .get(&reservation.permit_id)
                .ok_or_else(invalid_record)?
                .to_runtime()?;
            Ok((record, true))
        })
    }

    fn transition(
        &self,
        transition: HostEffectTransition,
    ) -> Result<HostEffectLedgerRecord, HostEffectLedgerError> {
        self.with_snapshot(|payload, replayed, _| {
            if transition.expected_head.generation != payload.generation
                || transition.expected_head.head_sha256 != payload.head_sha256
            {
                return Err(stale_head());
            }
            let current = replayed.records.get(&transition.permit_id).ok_or_else(|| {
                HostEffectLedgerError::new(HostEffectLedgerErrorId::InvalidRecord)
            })?;
            if current.state != transition.expected_state
                || !allowed_transition(transition.expected_state, transition.next_state)
            {
                return Err(invalid_transition());
            }
            let requires_outcome = matches!(
                transition.next_state,
                HostEffectState::Settled | HostEffectState::Failed | HostEffectState::Ambiguous
            );
            if transition.outcome_sha256.is_some() != requires_outcome
                || transition
                    .outcome_sha256
                    .as_ref()
                    .is_some_and(|value| !is_digest(value))
            {
                return Err(invalid_transition());
            }
            let event = transition_event(payload, &transition)?;
            payload.events.push(event);
            payload.generation += 1;
            payload.head_sha256 = payload
                .events
                .last()
                .ok_or_else(invalid_record)?
                .current_head_sha256
                .clone();
            let next = replay(payload)?;
            let record = next
                .records
                .get(&transition.permit_id)
                .ok_or_else(invalid_record)?
                .to_runtime()?;
            Ok((record, true))
        })
    }

    fn read(
        &self,
        permit_id: &str,
    ) -> Result<Option<HostEffectLedgerRecord>, HostEffectLedgerError> {
        if !is_digest(permit_id) {
            return Err(invalid_record());
        }
        self.with_snapshot(|_, replayed, _| {
            Ok((
                replayed
                    .records
                    .get(permit_id)
                    .map(CurrentRecord::to_runtime)
                    .transpose()?,
                false,
            ))
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SignedSnapshot {
    payload: SnapshotPayload,
    mac_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SnapshotPayload {
    schema: String,
    ledger_id: String,
    ledger_key_id: String,
    lock_identity: FileIdentity,
    generation: u64,
    head_sha256: String,
    events: Vec<PersistedEvent>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PersistedEvent {
    schema: String,
    generation: u64,
    prior_head_sha256: String,
    current_head_sha256: String,
    permit_id: String,
    reservation: Option<PersistedReservation>,
    expected_state: Option<String>,
    next_state: String,
    outcome_sha256: Option<String>,
    record_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PersistedReservation {
    issuer_id: String,
    ledger_id: String,
    key_id: String,
    permit_id: String,
    semantic_key_sha256: String,
    nonce_sha256: String,
    binding_sha256: String,
    expected_head_sha256: String,
    issued_at_unix_ms: u64,
    expires_at_unix_ms: u64,
    lifecycle_record: Option<crate::plugin_product::lifecycle::HostLifecycleRecord>,
    lifecycle_record_sha256: Option<String>,
}

#[derive(Default)]
struct ReplayState {
    records: BTreeMap<String, CurrentRecord>,
    nonces: BTreeSet<String>,
    semantic_keys: BTreeSet<String>,
}

#[derive(Clone)]
struct CurrentRecord {
    reservation: PersistedReservation,
    state: HostEffectState,
    record_sha256: String,
    prior_head: HostEffectLedgerHead,
    current_head: HostEffectLedgerHead,
    outcome_sha256: Option<String>,
}
