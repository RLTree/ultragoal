pub(super) const EXACT_OUTPUT_LIMIT_BYTES: usize = 1024 * 1024;
const MAX_TIMEOUT_MS: u64 = 5 * 60 * 1000;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum HostEffectExecutorErrorId {
    UnsupportedPlatform,
    InvalidPolicy,
    EnvironmentInjection,
    InvalidTargetRoot,
    TargetSubstitution,
    UnsafeObject,
    PathSwap,
    ExecutableMutation,
    LedgerSubstitution,
    LedgerRollback,
    Replay,
    TempCollision,
    RenameRace,
    SyncFailure,
    ProcessSpawnFailed,
    ProcessFailed,
    OutputOverflow,
    Timeout,
    Cancelled,
    PartialAcknowledgement,
    FalsePassReceipt,
    RecoveryRequired,
    Io,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum HostEffectTerminalRecoveryClassification {
    StillInFlight,
    TerminalCommittedAndVerified,
    TerminalCommittedButUnverifiable,
    LedgerObservationRejected,
    LedgerObservationUnavailable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum HostEffectPostReservationLedgerClassification {
    StillInFlight,
    TerminalObserved,
    ObservationRejected,
    ObservationUnavailable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum HostEffectPostReservationPublicationClassification {
    NoPublicationEvidence,
    PublicationEvidenceUnavailable,
    PublicationIdentityOnlyCurrentObservationUnavailable,
    PriorObservationCurrentObservationUnavailable,
}

pub(crate) struct HostEffectExecutorFailure {
    id: HostEffectExecutorErrorId,
    terminal_state: Option<HostEffectState>,
    recovery: Option<Box<HostEffectRecoveryHandoff>>,
}

impl std::fmt::Debug for HostEffectExecutorFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("HostEffectExecutorFailure")
            .field("id", &self.id)
            .field("terminal_state", &self.terminal_state)
            .field("has_recovery_handoff", &self.recovery.is_some())
            .finish()
    }
}

impl HostEffectExecutorFailure {
    pub(super) const fn new(id: HostEffectExecutorErrorId) -> Self {
        Self {
            id,
            terminal_state: None,
            recovery: None,
        }
    }

    pub(super) fn with_recovery(
        id: HostEffectExecutorErrorId,
        terminal_state: Option<HostEffectState>,
        recovery: HostEffectRecoveryHandoff,
    ) -> Self {
        Self {
            id,
            terminal_state,
            recovery: Some(Box::new(recovery)),
        }
    }

    pub(crate) const fn id(&self) -> HostEffectExecutorErrorId {
        self.id
    }

    pub(crate) const fn terminal_state(&self) -> Option<HostEffectState> {
        self.terminal_state
    }

    pub(crate) fn recovery(&self) -> Option<&HostEffectRecoveryHandoff> {
        self.recovery.as_deref()
    }
}

#[derive(Clone)]
pub(crate) enum HostEffectRecoveryHandoff {
    Publication {
        effect_identity_sha256: String,
        permit_id: String,
        ledger_head: HostEffectLedgerHead,
        observation: PublicationInventoryObservation,
        classification: PublicationClassification,
        originating_error_ids: Vec<HostEffectExecutorErrorId>,
        binding_sha256: String,
    },
    TerminalTransition {
        effect_identity_sha256: String,
        permit_id: String,
        ledger_head: HostEffectLedgerHead,
        ledger_record: HostEffectLedgerRecord,
        exact_current_ledger_observation: bool,
        outcome: HostEffectOutcome,
        originating_error_ids: Vec<HostEffectExecutorErrorId>,
        classification: HostEffectTerminalRecoveryClassification,
        binding_sha256: String,
    },
    PostReservation {
        effect_identity_sha256: String,
        permit_id: String,
        reservation_ledger_head: HostEffectLedgerHead,
        reservation_ledger_record: HostEffectLedgerRecord,
        ledger_head: HostEffectLedgerHead,
        ledger_record: HostEffectLedgerRecord,
        exact_current_ledger_observation: bool,
        publication_identity_sha256: Option<String>,
        prior_publication_observation: Option<PublicationInventoryObservation>,
        exact_current_publication_observation: bool,
        originating_error_ids: Vec<HostEffectExecutorErrorId>,
        ledger_classification: HostEffectPostReservationLedgerClassification,
        publication_classification: HostEffectPostReservationPublicationClassification,
        binding_sha256: String,
    },
}
