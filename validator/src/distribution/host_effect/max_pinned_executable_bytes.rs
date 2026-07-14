const MAX_PINNED_EXECUTABLE_BYTES: u64 = 512 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum HostEffectState {
    Reserved,
    InFlight,
    Settled,
    Failed,
    Ambiguous,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct HostEffectLedgerHead {
    generation: u64,
    head_sha256: String,
}

impl HostEffectLedgerHead {
    pub(in crate::distribution::host_effect) fn new(
        generation: u64,
        head_sha256: String,
    ) -> Result<Self, HostEffectLedgerError> {
        if !is_digest(&head_sha256) {
            return Err(HostEffectLedgerError::new(
                HostEffectLedgerErrorId::InvalidRecord,
            ));
        }
        Ok(Self {
            generation,
            head_sha256,
        })
    }

    pub(crate) const fn generation(&self) -> u64 {
        self.generation
    }

    pub(crate) fn head_sha256(&self) -> &str {
        &self.head_sha256
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct HostEffectReservation {
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
}

impl HostEffectReservation {
    pub(in crate::distribution::host_effect) fn from_permit(permit: &HostEffectPermit) -> Self {
        Self {
            issuer_id: permit.issuer_id().to_owned(),
            ledger_id: permit.ledger_id().to_owned(),
            key_id: permit.key_id().to_owned(),
            permit_id: permit.permit_id().to_owned(),
            semantic_key_sha256: permit.semantic_key_sha256().to_owned(),
            nonce_sha256: permit.nonce_sha256().to_owned(),
            binding_sha256: permit.binding_sha256().to_owned(),
            expected_head_sha256: permit.binding().expected_head_sha256.clone(),
            issued_at_unix_ms: permit.binding().issued_at_unix_ms,
            expires_at_unix_ms: permit.binding().expires_at_unix_ms,
        }
    }

    pub(crate) fn permit_id(&self) -> &str {
        &self.permit_id
    }

    pub(crate) fn semantic_key_sha256(&self) -> &str {
        &self.semantic_key_sha256
    }

    pub(crate) fn expected_head_sha256(&self) -> &str {
        &self.expected_head_sha256
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct HostEffectLedgerRecord {
    reservation: HostEffectReservation,
    state: HostEffectState,
    record_sha256: String,
    prior_head: HostEffectLedgerHead,
    current_head: HostEffectLedgerHead,
    outcome_sha256: Option<String>,
}

impl HostEffectLedgerRecord {
    pub(crate) fn reservation(&self) -> &HostEffectReservation {
        &self.reservation
    }

    pub(crate) const fn state(&self) -> HostEffectState {
        self.state
    }

    pub(crate) fn current_head(&self) -> &HostEffectLedgerHead {
        &self.current_head
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct HostEffectTransition {
    permit_id: String,
    expected_state: HostEffectState,
    next_state: HostEffectState,
    expected_head: HostEffectLedgerHead,
    outcome_sha256: Option<String>,
}

impl HostEffectTransition {
    pub(in crate::distribution::host_effect) fn new(
        permit_id: String,
        expected_state: HostEffectState,
        next_state: HostEffectState,
        expected_head: HostEffectLedgerHead,
        outcome_sha256: Option<String>,
    ) -> Result<Self, HostEffectLedgerError> {
        let requires_outcome = matches!(
            next_state,
            HostEffectState::Settled | HostEffectState::Failed | HostEffectState::Ambiguous
        );
        if !is_digest(&permit_id)
            || outcome_sha256.as_ref().is_some_and(|row| !is_digest(row))
            || outcome_sha256.is_some() != requires_outcome
            || !allowed_transition(expected_state, next_state)
        {
            return Err(HostEffectLedgerError::new(
                HostEffectLedgerErrorId::InvalidTransition,
            ));
        }
        Ok(Self {
            permit_id,
            expected_state,
            next_state,
            expected_head,
            outcome_sha256,
        })
    }
}

/// Cross-process implementations must reserve nonce and semantic key together,
/// publish transitions with compare-and-swap head semantics, fsync before
/// acknowledgement, and classify any uncertain started effect as Ambiguous.
pub(crate) trait DurableHostEffectLedger: Send + Sync {
    fn head(&self) -> Result<HostEffectLedgerHead, HostEffectLedgerError>;
    fn reserve(
        &self,
        reservation: HostEffectReservation,
    ) -> Result<HostEffectLedgerRecord, HostEffectLedgerError>;
    fn transition(
        &self,
        transition: HostEffectTransition,
    ) -> Result<HostEffectLedgerRecord, HostEffectLedgerError>;
    fn read(
        &self,
        permit_id: &str,
    ) -> Result<Option<HostEffectLedgerRecord>, HostEffectLedgerError>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum HostEffectLedgerErrorId {
    InvalidRecord,
    InvalidTransition,
    StaleHead,
    Replay,
    Tampered,
    Io,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct HostEffectLedgerError {
    id: HostEffectLedgerErrorId,
}
