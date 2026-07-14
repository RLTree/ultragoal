impl CurrentRecord {
    fn to_runtime(&self) -> Result<HostEffectLedgerRecord, HostEffectLedgerError> {
        Ok(HostEffectLedgerRecord {
            reservation: self.reservation.to_runtime()?,
            state: self.state,
            record_sha256: self.record_sha256.clone(),
            prior_head: self.prior_head.clone(),
            current_head: self.current_head.clone(),
            outcome_sha256: self.outcome_sha256.clone(),
        })
    }
}

impl PersistedReservation {
    fn from_runtime(value: &HostEffectReservation) -> Self {
        Self {
            issuer_id: value.issuer_id.clone(),
            ledger_id: value.ledger_id.clone(),
            key_id: value.key_id.clone(),
            permit_id: value.permit_id.clone(),
            semantic_key_sha256: value.semantic_key_sha256.clone(),
            nonce_sha256: value.nonce_sha256.clone(),
            binding_sha256: value.binding_sha256.clone(),
            expected_head_sha256: value.expected_head_sha256.clone(),
            issued_at_unix_ms: value.issued_at_unix_ms,
            expires_at_unix_ms: value.expires_at_unix_ms,
        }
    }

    fn to_runtime(&self) -> Result<HostEffectReservation, HostEffectLedgerError> {
        validate_persisted_reservation(self)?;
        Ok(HostEffectReservation {
            issuer_id: self.issuer_id.clone(),
            ledger_id: self.ledger_id.clone(),
            key_id: self.key_id.clone(),
            permit_id: self.permit_id.clone(),
            semantic_key_sha256: self.semantic_key_sha256.clone(),
            nonce_sha256: self.nonce_sha256.clone(),
            binding_sha256: self.binding_sha256.clone(),
            expected_head_sha256: self.expected_head_sha256.clone(),
            issued_at_unix_ms: self.issued_at_unix_ms,
            expires_at_unix_ms: self.expires_at_unix_ms,
        })
    }
}

fn initial_payload(
    ledger_id: &str,
    key_id: &str,
    lock_identity: FileIdentity,
) -> Result<SnapshotPayload, HostEffectLedgerError> {
    #[derive(Serialize)]
    struct InitialHead<'a> {
        schema: &'static str,
        ledger_id: &'a str,
        ledger_key_id: &'a str,
        lock_identity: FileIdentity,
    }
    let head_sha256 = digest_json(&InitialHead {
        schema: INITIAL_HEAD_SCHEMA,
        ledger_id,
        ledger_key_id: key_id,
        lock_identity,
    })?;
    Ok(SnapshotPayload {
        schema: LEDGER_SCHEMA.to_owned(),
        ledger_id: ledger_id.to_owned(),
        ledger_key_id: key_id.to_owned(),
        lock_identity,
        generation: 0,
        head_sha256,
        events: Vec::new(),
    })
}

fn reserve_event(
    payload: &SnapshotPayload,
    reservation: &HostEffectReservation,
) -> Result<PersistedEvent, HostEffectLedgerError> {
    event(
        payload,
        reservation.permit_id.clone(),
        Some(PersistedReservation::from_runtime(reservation)),
        None,
        HostEffectState::Reserved,
        None,
    )
}

fn transition_event(
    payload: &SnapshotPayload,
    transition: &HostEffectTransition,
) -> Result<PersistedEvent, HostEffectLedgerError> {
    event(
        payload,
        transition.permit_id.clone(),
        None,
        Some(transition.expected_state),
        transition.next_state,
        transition.outcome_sha256.clone(),
    )
}

fn event(
    payload: &SnapshotPayload,
    permit_id: String,
    reservation: Option<PersistedReservation>,
    expected_state: Option<HostEffectState>,
    next_state: HostEffectState,
    outcome_sha256: Option<String>,
) -> Result<PersistedEvent, HostEffectLedgerError> {
    let generation = payload
        .generation
        .checked_add(1)
        .ok_or_else(invalid_record)?;
    let expected_state = expected_state.map(state_name).map(str::to_owned);
    let next_state = state_name(next_state).to_owned();
    #[derive(Serialize)]
    struct RecordPreimage<'a> {
        schema: &'static str,
        ledger_id: &'a str,
        ledger_key_id: &'a str,
        lock_identity: FileIdentity,
        generation: u64,
        prior_head_sha256: &'a str,
        permit_id: &'a str,
        reservation: &'a Option<PersistedReservation>,
        expected_state: &'a Option<String>,
        next_state: &'a str,
        outcome_sha256: &'a Option<String>,
    }
    let record_sha256 = digest_json(&RecordPreimage {
        schema: EVENT_SCHEMA,
        ledger_id: &payload.ledger_id,
        ledger_key_id: &payload.ledger_key_id,
        lock_identity: payload.lock_identity,
        generation,
        prior_head_sha256: &payload.head_sha256,
        permit_id: &permit_id,
        reservation: &reservation,
        expected_state: &expected_state,
        next_state: &next_state,
        outcome_sha256: &outcome_sha256,
    })?;
    #[derive(Serialize)]
    struct HeadPreimage<'a> {
        schema: &'static str,
        ledger_id: &'a str,
        ledger_key_id: &'a str,
        lock_identity: FileIdentity,
        generation: u64,
        prior_head_sha256: &'a str,
        permit_id: &'a str,
        record_sha256: &'a str,
    }
    let current_head_sha256 = digest_json(&HeadPreimage {
        schema: "harness-ultragoal.host-effect-ledger-head.v2",
        ledger_id: &payload.ledger_id,
        ledger_key_id: &payload.ledger_key_id,
        lock_identity: payload.lock_identity,
        generation,
        prior_head_sha256: &payload.head_sha256,
        permit_id: &permit_id,
        record_sha256: &record_sha256,
    })?;
    Ok(PersistedEvent {
        schema: EVENT_SCHEMA.to_owned(),
        generation,
        prior_head_sha256: payload.head_sha256.clone(),
        current_head_sha256,
        permit_id,
        reservation,
        expected_state,
        next_state,
        outcome_sha256,
        record_sha256,
    })
}
