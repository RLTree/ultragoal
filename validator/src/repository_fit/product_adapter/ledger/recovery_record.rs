use super::*;

pub(crate) const RECOVERY_INTENT_SCHEMA: &str = "RepositoryFitRecoveryIntent-v2";

#[derive(Serialize)]
pub(crate) struct CanonicalRecoveryIntent<'a> {
    pub(crate) schema_version: &'static str,
    pub(crate) recovery: &'a RecoveryTargetSpec,
}

pub(crate) fn canonical_recovery_intent_bytes(recovery: &RecoveryTargetSpec) -> Option<Vec<u8>> {
    serde_json::to_vec(&CanonicalRecoveryIntent {
        schema_version: RECOVERY_INTENT_SCHEMA,
        recovery,
    })
    .ok()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RepositoryFitLedgerState {
    Reserved,
    EffectStarted,
    Committed,
    RolledBack,
    Rejected,
    Interrupted,
    Ambiguous,
}

impl RepositoryFitLedgerState {
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Reserved => "reserved",
            Self::EffectStarted => "effect_started",
            Self::Committed => "committed",
            Self::RolledBack => "rolled_back",
            Self::Rejected => "rejected",
            Self::Interrupted => "interrupted",
            Self::Ambiguous => "ambiguous",
        }
    }

    pub(crate) const fn terminal(self) -> bool {
        !matches!(self, Self::Reserved | Self::EffectStarted)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RecoveryTargetSpec {
    pub(crate) request_id: String,
    pub(crate) root_binding: String,
    pub(crate) ancestors: ManagedAncestorContract,
    pub(crate) rows: Vec<RecoveryTargetRow>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RecoveryTargetRow {
    pub(crate) path: String,
    pub(crate) pre_sha256: Option<String>,
    pub(crate) pre_mode: Option<u32>,
    pub(crate) post_sha256: String,
    pub(crate) post_mode: u32,
}

pub(crate) struct ReservationRequest<'a> {
    pub(crate) binding_sha256: &'a str,
    pub(crate) semantic_effect_id: &'a str,
    pub(crate) target_scope_id: &'a str,
    pub(crate) permit_id: &'a str,
    pub(crate) nonce_sha256: &'a str,
    pub(crate) recovery_intent_sha256: &'a str,
    pub(crate) issued_tick: u64,
    pub(crate) expires_tick: u64,
    pub(crate) recovery: &'a RecoveryTargetSpec,
}

pub(crate) struct ReservationToken {
    pub(crate) reservation_id: String,
    pub(crate) binding_sha256: String,
    pub(crate) semantic_effect_id: String,
    pub(crate) target_scope_id: String,
    pub(crate) permit_id: String,
    pub(crate) nonce_sha256: String,
    pub(crate) recovery_intent_sha256: String,
}

impl std::fmt::Debug for ReservationToken {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ReservationToken")
            .field("reservation", &"[bound]")
            .field("binding", &"[bound]")
            .field("semantic_effect", &"[bound]")
            .field("target_scope", &"[bound]")
            .field("permit", &"[bound]")
            .field("nonce", &"[redacted]")
            .finish()
    }
}

pub(crate) struct ExistingReservation {
    pub(crate) reservation_id: String,
    pub(crate) state: RepositoryFitLedgerState,
    pub(crate) expires_tick: u64,
    pub(crate) recovery_intent_sha256: String,
    pub(crate) recovery: RecoveryTargetSpec,
    pub(crate) terminal_sha256: Option<String>,
}

impl ExistingReservation {
    pub(crate) const fn state(&self) -> RepositoryFitLedgerState {
        self.state
    }

    pub(crate) const fn expires_tick(&self) -> u64 {
        self.expires_tick
    }

    pub(crate) fn request_id(&self) -> &str {
        &self.recovery.request_id
    }

    pub(crate) fn matches_intent(
        &self,
        recovery_intent_sha256: &str,
        recovery: &RecoveryTargetSpec,
    ) -> bool {
        self.recovery_intent_sha256 == recovery_intent_sha256 && self.recovery == *recovery
    }

    #[cfg(test)]
    pub(crate) fn terminal_sha256(&self) -> Option<&str> {
        self.terminal_sha256.as_deref()
    }
}

pub(crate) struct RecoveryTerminal {
    pub(crate) state: RepositoryFitLedgerState,
    pub(crate) terminal_sha256: String,
    pub(crate) error_id: Option<AdapterErrorId>,
}

pub(crate) struct EffectOwner<'a> {
    #[cfg(target_vendor = "apple")]
    pub(crate) inner: supported::EffectOwner<'a>,
    #[cfg(not(target_vendor = "apple"))]
    pub(crate) _marker: std::marker::PhantomData<&'a ()>,
}

pub(crate) enum ReservationDecision {
    Acquired(ReservationToken),
    Existing(ExistingReservation),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LedgerErrorId {
    UnsupportedHost,
    InvalidStore,
    Tampered,
    Replay,
    ActiveLease,
    InvalidTransition,
    Io,
}

#[derive(Debug)]
pub(crate) struct LedgerError {
    pub(crate) id: LedgerErrorId,
}
