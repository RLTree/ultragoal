use super::super::lifecycle::{
    PublicationAcknowledgementIdentity, PublicationClassification, PublicationClassificationId,
    PublicationInventoryObservation,
};
use super::super::{
    HostEffectLedgerHead, HostEffectLedgerRecord, HostEffectOutcome, HostEffectState, is_digest,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

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
pub(crate) enum HostEffectPostPublicationRecoveryClassification {
    CommittedBeforeTerminalTransitionObservationUnavailable,
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
    PostPublicationTerminalTransition {
        effect_identity_sha256: String,
        permit_id: String,
        prior_ledger_head: HostEffectLedgerHead,
        publication_identity_sha256: String,
        prior_publication_observation: PublicationInventoryObservation,
        exact_current_publication_observation: bool,
        originating_error_ids: Vec<HostEffectExecutorErrorId>,
        classification: HostEffectPostPublicationRecoveryClassification,
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

impl std::fmt::Debug for HostEffectRecoveryHandoff {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Publication {
                effect_identity_sha256,
                permit_id,
                ledger_head,
                classification,
                originating_error_ids,
                binding_sha256,
                ..
            } => formatter
                .debug_struct("HostEffectRecoveryHandoff::Publication")
                .field("effect_identity_sha256", effect_identity_sha256)
                .field("permit_id", permit_id)
                .field("ledger_head", ledger_head)
                .field("classification", &classification.id())
                .field("originating_error_ids", originating_error_ids)
                .field("binding_sha256", binding_sha256)
                .finish_non_exhaustive(),
            Self::TerminalTransition {
                effect_identity_sha256,
                permit_id,
                ledger_head,
                exact_current_ledger_observation,
                originating_error_ids,
                classification,
                binding_sha256,
                ..
            } => formatter
                .debug_struct("HostEffectRecoveryHandoff::TerminalTransition")
                .field("effect_identity_sha256", effect_identity_sha256)
                .field("permit_id", permit_id)
                .field("ledger_head", ledger_head)
                .field(
                    "exact_current_ledger_observation",
                    exact_current_ledger_observation,
                )
                .field("originating_error_ids", originating_error_ids)
                .field("classification", classification)
                .field("binding_sha256", binding_sha256)
                .finish_non_exhaustive(),
            Self::PostPublicationTerminalTransition {
                effect_identity_sha256,
                permit_id,
                prior_ledger_head,
                publication_identity_sha256,
                exact_current_publication_observation,
                originating_error_ids,
                classification,
                binding_sha256,
                ..
            } => formatter
                .debug_struct("HostEffectRecoveryHandoff::PostPublicationTerminalTransition")
                .field("effect_identity_sha256", effect_identity_sha256)
                .field("permit_id", permit_id)
                .field("prior_ledger_head", prior_ledger_head)
                .field("publication_identity_sha256", publication_identity_sha256)
                .field(
                    "exact_current_publication_observation",
                    exact_current_publication_observation,
                )
                .field("originating_error_ids", originating_error_ids)
                .field("classification", classification)
                .field("binding_sha256", binding_sha256)
                .finish_non_exhaustive(),
            Self::PostReservation {
                effect_identity_sha256,
                permit_id,
                ledger_head,
                exact_current_ledger_observation,
                publication_identity_sha256,
                exact_current_publication_observation,
                originating_error_ids,
                ledger_classification,
                publication_classification,
                binding_sha256,
                ..
            } => formatter
                .debug_struct("HostEffectRecoveryHandoff::PostReservation")
                .field("effect_identity_sha256", effect_identity_sha256)
                .field("permit_id", permit_id)
                .field("ledger_head", ledger_head)
                .field(
                    "exact_current_ledger_observation",
                    exact_current_ledger_observation,
                )
                .field("publication_identity_sha256", publication_identity_sha256)
                .field(
                    "exact_current_publication_observation",
                    exact_current_publication_observation,
                )
                .field("originating_error_ids", originating_error_ids)
                .field("ledger_classification", ledger_classification)
                .field("publication_classification", publication_classification)
                .field("binding_sha256", binding_sha256)
                .finish_non_exhaustive(),
        }
    }
}

impl HostEffectRecoveryHandoff {
    pub(super) fn publication(
        effect_identity_sha256: String,
        permit_id: String,
        ledger_head: HostEffectLedgerHead,
        observation: PublicationInventoryObservation,
        classification: PublicationClassification,
        originating_error_ids: Vec<HostEffectExecutorErrorId>,
    ) -> Self {
        let mut handoff = Self::Publication {
            effect_identity_sha256,
            permit_id,
            ledger_head,
            observation,
            classification,
            originating_error_ids,
            binding_sha256: String::new(),
        };
        let binding_sha256 = handoff.publication_binding_sha256();
        if let Self::Publication {
            binding_sha256: stored,
            ..
        } = &mut handoff
        {
            *stored = binding_sha256;
        }
        handoff
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn terminal_transition(
        effect_identity_sha256: String,
        permit_id: String,
        ledger_head: HostEffectLedgerHead,
        ledger_record: HostEffectLedgerRecord,
        exact_current_ledger_observation: bool,
        outcome: HostEffectOutcome,
        originating_error_ids: Vec<HostEffectExecutorErrorId>,
        classification: HostEffectTerminalRecoveryClassification,
    ) -> Self {
        let mut handoff = Self::TerminalTransition {
            effect_identity_sha256,
            permit_id,
            ledger_head,
            ledger_record,
            exact_current_ledger_observation,
            outcome,
            originating_error_ids,
            classification,
            binding_sha256: String::new(),
        };
        let binding_sha256 = handoff.terminal_binding_sha256();
        if let Self::TerminalTransition {
            binding_sha256: stored,
            ..
        } = &mut handoff
        {
            *stored = binding_sha256;
        }
        handoff
    }

    pub(super) fn post_publication_terminal_transition_observation_unavailable(
        effect_identity_sha256: String,
        permit_id: String,
        prior_ledger_head: HostEffectLedgerHead,
        publication_identity_sha256: String,
        prior_publication_observation: PublicationInventoryObservation,
        originating_error_ids: Vec<HostEffectExecutorErrorId>,
    ) -> Self {
        let mut handoff = Self::PostPublicationTerminalTransition {
            effect_identity_sha256,
            permit_id,
            prior_ledger_head,
            publication_identity_sha256,
            prior_publication_observation,
            exact_current_publication_observation: false,
            originating_error_ids,
            classification: HostEffectPostPublicationRecoveryClassification::CommittedBeforeTerminalTransitionObservationUnavailable,
            binding_sha256: String::new(),
        };
        let binding_sha256 = handoff.post_publication_binding_sha256();
        if let Self::PostPublicationTerminalTransition {
            binding_sha256: stored,
            ..
        } = &mut handoff
        {
            *stored = binding_sha256;
        }
        handoff
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn post_reservation(
        effect_identity_sha256: String,
        permit_id: String,
        reservation_ledger_head: HostEffectLedgerHead,
        reservation_ledger_record: HostEffectLedgerRecord,
        ledger_head: HostEffectLedgerHead,
        ledger_record: HostEffectLedgerRecord,
        exact_current_ledger_observation: bool,
        publication_identity_sha256: Option<String>,
        prior_publication_observation: Option<PublicationInventoryObservation>,
        originating_error_ids: Vec<HostEffectExecutorErrorId>,
        ledger_classification: HostEffectPostReservationLedgerClassification,
        publication_classification: HostEffectPostReservationPublicationClassification,
    ) -> Self {
        let mut handoff = Self::PostReservation {
            effect_identity_sha256,
            permit_id,
            reservation_ledger_head,
            reservation_ledger_record,
            ledger_head,
            ledger_record,
            exact_current_ledger_observation,
            publication_identity_sha256,
            prior_publication_observation,
            exact_current_publication_observation: false,
            originating_error_ids,
            ledger_classification,
            publication_classification,
            binding_sha256: String::new(),
        };
        let binding_sha256 = handoff.post_reservation_binding_sha256();
        if let Self::PostReservation {
            binding_sha256: stored,
            ..
        } = &mut handoff
        {
            *stored = binding_sha256;
        }
        handoff
    }

    pub(crate) fn effect_identity_sha256(&self) -> &str {
        match self {
            Self::Publication {
                effect_identity_sha256,
                ..
            }
            | Self::TerminalTransition {
                effect_identity_sha256,
                ..
            }
            | Self::PostPublicationTerminalTransition {
                effect_identity_sha256,
                ..
            }
            | Self::PostReservation {
                effect_identity_sha256,
                ..
            } => effect_identity_sha256,
        }
    }

    pub(crate) fn permit_id(&self) -> &str {
        match self {
            Self::Publication { permit_id, .. }
            | Self::TerminalTransition { permit_id, .. }
            | Self::PostPublicationTerminalTransition { permit_id, .. }
            | Self::PostReservation { permit_id, .. } => permit_id,
        }
    }

    pub(crate) fn ledger_head(&self) -> &HostEffectLedgerHead {
        match self {
            Self::Publication { ledger_head, .. }
            | Self::TerminalTransition { ledger_head, .. } => ledger_head,
            Self::PostPublicationTerminalTransition {
                prior_ledger_head, ..
            } => prior_ledger_head,
            Self::PostReservation { ledger_head, .. } => ledger_head,
        }
    }

    pub(crate) fn observation(&self) -> Option<&PublicationInventoryObservation> {
        match self {
            Self::Publication { observation, .. } => Some(observation),
            Self::TerminalTransition { .. }
            | Self::PostPublicationTerminalTransition { .. }
            | Self::PostReservation { .. } => None,
        }
    }

    pub(crate) fn prior_publication_observation(&self) -> Option<&PublicationInventoryObservation> {
        match self {
            Self::PostPublicationTerminalTransition {
                prior_publication_observation,
                ..
            } => Some(prior_publication_observation),
            Self::PostReservation {
                prior_publication_observation,
                ..
            } => prior_publication_observation.as_ref(),
            Self::Publication { .. } | Self::TerminalTransition { .. } => None,
        }
    }

    pub(crate) fn publication_identity_sha256(&self) -> Option<&str> {
        match self {
            Self::PostPublicationTerminalTransition {
                publication_identity_sha256,
                ..
            } => Some(publication_identity_sha256),
            Self::PostReservation {
                publication_identity_sha256,
                ..
            } => publication_identity_sha256.as_deref(),
            Self::Publication { .. } | Self::TerminalTransition { .. } => None,
        }
    }

    pub(crate) fn classification(&self) -> Option<&PublicationClassification> {
        match self {
            Self::Publication { classification, .. } => Some(classification),
            Self::TerminalTransition { .. }
            | Self::PostPublicationTerminalTransition { .. }
            | Self::PostReservation { .. } => None,
        }
    }

    pub(crate) fn post_publication_classification(
        &self,
    ) -> Option<HostEffectPostPublicationRecoveryClassification> {
        match self {
            Self::PostPublicationTerminalTransition { classification, .. } => Some(*classification),
            Self::Publication { .. }
            | Self::TerminalTransition { .. }
            | Self::PostReservation { .. } => None,
        }
    }

    pub(crate) fn terminal_classification(
        &self,
    ) -> Option<HostEffectTerminalRecoveryClassification> {
        match self {
            Self::Publication { .. }
            | Self::PostPublicationTerminalTransition { .. }
            | Self::PostReservation { .. } => None,
            Self::TerminalTransition { classification, .. } => Some(*classification),
        }
    }

    pub(crate) fn ledger_record(&self) -> Option<&HostEffectLedgerRecord> {
        match self {
            Self::Publication { .. } | Self::PostPublicationTerminalTransition { .. } => None,
            Self::TerminalTransition { ledger_record, .. } => Some(ledger_record),
            Self::PostReservation { ledger_record, .. } => Some(ledger_record),
        }
    }

    pub(crate) fn outcome(&self) -> Option<&HostEffectOutcome> {
        match self {
            Self::Publication { .. }
            | Self::PostPublicationTerminalTransition { .. }
            | Self::PostReservation { .. } => None,
            Self::TerminalTransition { outcome, .. } => Some(outcome),
        }
    }

    pub(crate) fn originating_error_id(&self) -> Option<HostEffectExecutorErrorId> {
        match self {
            Self::Publication {
                originating_error_ids,
                ..
            }
            | Self::TerminalTransition {
                originating_error_ids,
                ..
            }
            | Self::PostPublicationTerminalTransition {
                originating_error_ids,
                ..
            }
            | Self::PostReservation {
                originating_error_ids,
                ..
            } => originating_error_ids.first().copied(),
        }
    }

    pub(crate) fn originating_error_ids(&self) -> &[HostEffectExecutorErrorId] {
        match self {
            Self::Publication {
                originating_error_ids,
                ..
            }
            | Self::TerminalTransition {
                originating_error_ids,
                ..
            }
            | Self::PostPublicationTerminalTransition {
                originating_error_ids,
                ..
            }
            | Self::PostReservation {
                originating_error_ids,
                ..
            } => originating_error_ids,
        }
    }

    pub(crate) fn post_reservation_ledger_classification(
        &self,
    ) -> Option<HostEffectPostReservationLedgerClassification> {
        match self {
            Self::PostReservation {
                ledger_classification,
                ..
            } => Some(*ledger_classification),
            _ => None,
        }
    }

    pub(crate) fn post_reservation_publication_classification(
        &self,
    ) -> Option<HostEffectPostReservationPublicationClassification> {
        match self {
            Self::PostReservation {
                publication_classification,
                ..
            } => Some(*publication_classification),
            _ => None,
        }
    }

    pub(crate) fn has_exact_current_ledger_observation(&self) -> bool {
        matches!(
            self,
            Self::TerminalTransition {
                exact_current_ledger_observation: true,
                ..
            } | Self::PostReservation {
                exact_current_ledger_observation: true,
                ..
            }
        )
    }

    pub(crate) fn has_exact_current_publication_observation(&self) -> bool {
        match self {
            Self::Publication { .. } => true,
            Self::TerminalTransition { .. } => false,
            Self::PostPublicationTerminalTransition {
                exact_current_publication_observation,
                ..
            }
            | Self::PostReservation {
                exact_current_publication_observation,
                ..
            } => *exact_current_publication_observation,
        }
    }

    pub(crate) fn binding_sha256(&self) -> Option<&str> {
        match self {
            Self::Publication { binding_sha256, .. }
            | Self::TerminalTransition { binding_sha256, .. }
            | Self::PostPublicationTerminalTransition { binding_sha256, .. }
            | Self::PostReservation { binding_sha256, .. } => Some(binding_sha256),
        }
    }

    pub(crate) fn verify_binding(&self) -> bool {
        match self {
            Self::Publication {
                permit_id,
                observation,
                classification,
                originating_error_ids,
                binding_sha256,
                ..
            } => {
                binding_sha256 == &self.publication_binding_sha256()
                    && is_digest(self.effect_identity_sha256())
                    && is_digest(permit_id)
                    && valid_originating_error_chain(originating_error_ids)
                    && observation
                        .classify()
                        .is_ok_and(|observed| observed == *classification)
            }
            Self::TerminalTransition {
                permit_id,
                ledger_head,
                ledger_record,
                exact_current_ledger_observation,
                outcome,
                originating_error_ids,
                classification,
                binding_sha256,
                ..
            } => {
                if binding_sha256 != &self.terminal_binding_sha256()
                    || !is_digest(self.effect_identity_sha256())
                    || !is_digest(permit_id)
                    || ledger_record.reservation().permit_id() != permit_id
                    || ledger_record.current_head() != ledger_head
                    || outcome.permit_id != *permit_id
                    || !valid_originating_error_chain(originating_error_ids)
                {
                    return false;
                }
                match classification {
                    HostEffectTerminalRecoveryClassification::StillInFlight => {
                        *exact_current_ledger_observation
                            && ledger_record.state() == HostEffectState::InFlight
                            && ledger_record.outcome_sha256.is_none()
                    }
                    HostEffectTerminalRecoveryClassification::TerminalCommittedAndVerified
                    | HostEffectTerminalRecoveryClassification::TerminalCommittedButUnverifiable => {
                        *exact_current_ledger_observation
                            && ledger_record.state() == outcome.state
                            && ledger_record.outcome_sha256.as_deref()
                                == Some(outcome.outcome_sha256.as_str())
                    }
                    HostEffectTerminalRecoveryClassification::LedgerObservationRejected => {
                        *exact_current_ledger_observation
                    }
                    HostEffectTerminalRecoveryClassification::LedgerObservationUnavailable => {
                        !*exact_current_ledger_observation
                    }
                }
            }
            Self::PostPublicationTerminalTransition {
                permit_id,
                publication_identity_sha256,
                prior_publication_observation,
                exact_current_publication_observation,
                originating_error_ids,
                classification,
                binding_sha256,
                ..
            } => {
                binding_sha256 == &self.post_publication_binding_sha256()
                    && is_digest(self.effect_identity_sha256())
                    && is_digest(permit_id)
                    && is_digest(publication_identity_sha256)
                    && !*exact_current_publication_observation
                    && valid_originating_error_chain(originating_error_ids)
                    && *classification
                        == HostEffectPostPublicationRecoveryClassification::CommittedBeforeTerminalTransitionObservationUnavailable
                    && prior_publication_observation
                        .classify()
                        .is_ok_and(|prior| {
                            prior.id()
                                == PublicationClassificationId::CommittedBeforeAcknowledgement
                        })
            }
            Self::PostReservation {
                permit_id,
                reservation_ledger_head,
                reservation_ledger_record,
                ledger_head,
                ledger_record,
                exact_current_ledger_observation,
                publication_identity_sha256,
                prior_publication_observation,
                exact_current_publication_observation,
                originating_error_ids,
                ledger_classification,
                publication_classification,
                binding_sha256,
                ..
            } => {
                if binding_sha256 != &self.post_reservation_binding_sha256()
                    || !is_digest(self.effect_identity_sha256())
                    || !is_digest(permit_id)
                    || reservation_ledger_record.reservation().permit_id() != permit_id
                    || reservation_ledger_record.state() != HostEffectState::InFlight
                    || reservation_ledger_record.current_head() != reservation_ledger_head
                    || ledger_record.reservation().permit_id() != permit_id
                    || *exact_current_publication_observation
                    || !valid_originating_error_chain(originating_error_ids)
                    || publication_identity_sha256
                        .as_ref()
                        .is_some_and(|identity| !is_digest(identity))
                {
                    return false;
                }
                let ledger_valid = match ledger_classification {
                    HostEffectPostReservationLedgerClassification::StillInFlight => {
                        *exact_current_ledger_observation
                            && ledger_record == reservation_ledger_record
                            && ledger_record.current_head() == ledger_head
                    }
                    HostEffectPostReservationLedgerClassification::TerminalObserved => {
                        *exact_current_ledger_observation
                            && ledger_record.reservation()
                                == reservation_ledger_record.reservation()
                            && ledger_record.current_head() == ledger_head
                            && matches!(
                                ledger_record.state(),
                                HostEffectState::Settled
                                    | HostEffectState::Failed
                                    | HostEffectState::Ambiguous
                            )
                    }
                    HostEffectPostReservationLedgerClassification::ObservationRejected => {
                        *exact_current_ledger_observation
                    }
                    HostEffectPostReservationLedgerClassification::ObservationUnavailable => {
                        !*exact_current_ledger_observation
                            && ledger_record == reservation_ledger_record
                            && ledger_head == reservation_ledger_head
                    }
                };
                let publication_valid = match publication_classification {
                    HostEffectPostReservationPublicationClassification::NoPublicationEvidence => {
                        publication_identity_sha256.is_none()
                            && prior_publication_observation.is_none()
                    }
                    HostEffectPostReservationPublicationClassification::PublicationEvidenceUnavailable => {
                        publication_identity_sha256.is_none()
                            && prior_publication_observation.is_none()
                    }
                    HostEffectPostReservationPublicationClassification::PublicationIdentityOnlyCurrentObservationUnavailable => {
                        publication_identity_sha256.is_some()
                            && prior_publication_observation.is_none()
                    }
                    HostEffectPostReservationPublicationClassification::PriorObservationCurrentObservationUnavailable => {
                        prior_publication_observation
                            .as_ref()
                            .is_some_and(|observation| observation.classify().is_ok())
                    }
                };
                ledger_valid && publication_valid
            }
        }
    }

    fn publication_binding_sha256(&self) -> String {
        let Self::Publication {
            effect_identity_sha256,
            permit_id,
            ledger_head,
            observation,
            classification,
            originating_error_ids,
            ..
        } = self
        else {
            return String::new();
        };
        #[derive(Serialize)]
        struct Binding<'a> {
            schema: &'static str,
            effect_identity_sha256: &'a str,
            permit_id: &'a str,
            ledger_head: &'a HostEffectLedgerHead,
            observation: &'a PublicationInventoryObservation,
            classification: &'a PublicationClassification,
            originating_error_ids: &'a [HostEffectExecutorErrorId],
        }
        digest_json(&Binding {
            schema: "harness-ultragoal.host-effect-publication-recovery.v2",
            effect_identity_sha256,
            permit_id,
            ledger_head,
            observation,
            classification,
            originating_error_ids,
        })
        .expect("publication recovery binding serialization is infallible")
    }

    fn terminal_binding_sha256(&self) -> String {
        let Self::TerminalTransition {
            effect_identity_sha256,
            permit_id,
            ledger_head,
            ledger_record,
            exact_current_ledger_observation,
            outcome,
            originating_error_ids,
            classification,
            ..
        } = self
        else {
            return String::new();
        };
        #[derive(Serialize)]
        struct Binding<'a> {
            schema: &'static str,
            effect_identity_sha256: &'a str,
            permit_id: &'a str,
            ledger_head: &'a HostEffectLedgerHead,
            ledger_record: &'a HostEffectLedgerRecord,
            exact_current_ledger_observation: bool,
            outcome: &'a HostEffectOutcome,
            originating_error_ids: &'a [HostEffectExecutorErrorId],
            classification: HostEffectTerminalRecoveryClassification,
        }
        // Every member uses a derived serializer over finite primitive values;
        // serialization cannot fail for a constructed recovery handoff.
        digest_json(&Binding {
            schema: "harness-ultragoal.host-effect-terminal-recovery.v2",
            effect_identity_sha256,
            permit_id,
            ledger_head,
            ledger_record,
            exact_current_ledger_observation: *exact_current_ledger_observation,
            outcome,
            originating_error_ids,
            classification: *classification,
        })
        .expect("terminal recovery binding serialization is infallible")
    }

    fn post_publication_binding_sha256(&self) -> String {
        let Self::PostPublicationTerminalTransition {
            effect_identity_sha256,
            permit_id,
            prior_ledger_head,
            publication_identity_sha256,
            prior_publication_observation,
            exact_current_publication_observation,
            originating_error_ids,
            classification,
            ..
        } = self
        else {
            return String::new();
        };
        #[derive(Serialize)]
        struct Binding<'a> {
            schema: &'static str,
            effect_identity_sha256: &'a str,
            permit_id: &'a str,
            prior_ledger_head: &'a HostEffectLedgerHead,
            publication_identity_sha256: &'a str,
            prior_publication_observation: &'a PublicationInventoryObservation,
            exact_current_publication_observation: bool,
            originating_error_ids: &'a [HostEffectExecutorErrorId],
            classification: HostEffectPostPublicationRecoveryClassification,
        }
        digest_json(&Binding {
            schema: "harness-ultragoal.host-effect-post-publication-terminal-recovery.v2",
            effect_identity_sha256,
            permit_id,
            prior_ledger_head,
            publication_identity_sha256,
            prior_publication_observation,
            exact_current_publication_observation: *exact_current_publication_observation,
            originating_error_ids,
            classification: *classification,
        })
        .expect("post-publication recovery binding serialization is infallible")
    }

    fn post_reservation_binding_sha256(&self) -> String {
        let Self::PostReservation {
            effect_identity_sha256,
            permit_id,
            reservation_ledger_head,
            reservation_ledger_record,
            ledger_head,
            ledger_record,
            exact_current_ledger_observation,
            publication_identity_sha256,
            prior_publication_observation,
            exact_current_publication_observation,
            originating_error_ids,
            ledger_classification,
            publication_classification,
            ..
        } = self
        else {
            return String::new();
        };
        #[derive(Serialize)]
        struct Binding<'a> {
            schema: &'static str,
            effect_identity_sha256: &'a str,
            permit_id: &'a str,
            reservation_ledger_head: &'a HostEffectLedgerHead,
            reservation_ledger_record: &'a HostEffectLedgerRecord,
            ledger_head: &'a HostEffectLedgerHead,
            ledger_record: &'a HostEffectLedgerRecord,
            exact_current_ledger_observation: bool,
            publication_identity_sha256: &'a Option<String>,
            prior_publication_observation: &'a Option<PublicationInventoryObservation>,
            exact_current_publication_observation: bool,
            originating_error_ids: &'a [HostEffectExecutorErrorId],
            ledger_classification: HostEffectPostReservationLedgerClassification,
            publication_classification: HostEffectPostReservationPublicationClassification,
        }
        digest_json(&Binding {
            schema: "harness-ultragoal.host-effect-post-reservation-recovery.v1",
            effect_identity_sha256,
            permit_id,
            reservation_ledger_head,
            reservation_ledger_record,
            ledger_head,
            ledger_record,
            exact_current_ledger_observation: *exact_current_ledger_observation,
            publication_identity_sha256,
            prior_publication_observation,
            exact_current_publication_observation: *exact_current_publication_observation,
            originating_error_ids,
            ledger_classification: *ledger_classification,
            publication_classification: *publication_classification,
        })
        .expect("post-reservation recovery binding serialization is infallible")
    }

    #[cfg(test)]
    pub(super) fn substitute_permit_without_rebinding_for_test(&mut self, permit_id: String) {
        match self {
            Self::Publication {
                permit_id: current, ..
            }
            | Self::TerminalTransition {
                permit_id: current, ..
            }
            | Self::PostPublicationTerminalTransition {
                permit_id: current, ..
            }
            | Self::PostReservation {
                permit_id: current, ..
            } => *current = permit_id,
        }
    }

    #[cfg(test)]
    pub(super) fn substitute_ledger_head_without_rebinding_for_test(
        &mut self,
        replacement: HostEffectLedgerHead,
    ) {
        match self {
            Self::Publication { ledger_head, .. }
            | Self::TerminalTransition { ledger_head, .. }
            | Self::PostReservation { ledger_head, .. } => *ledger_head = replacement,
            Self::PostPublicationTerminalTransition {
                prior_ledger_head, ..
            } => *prior_ledger_head = replacement,
        }
    }

    #[cfg(test)]
    pub(super) fn substitute_originating_error_ids_without_rebinding_for_test(
        &mut self,
        replacement: Vec<HostEffectExecutorErrorId>,
    ) {
        match self {
            Self::Publication {
                originating_error_ids,
                ..
            }
            | Self::TerminalTransition {
                originating_error_ids,
                ..
            }
            | Self::PostPublicationTerminalTransition {
                originating_error_ids,
                ..
            }
            | Self::PostReservation {
                originating_error_ids,
                ..
            } => *originating_error_ids = replacement,
        }
    }
}

fn valid_originating_error_chain(chain: &[HostEffectExecutorErrorId]) -> bool {
    const MAX_ORIGINATING_ERROR_CHAIN_LENGTH: usize = 4;
    !chain.is_empty()
        && chain.len() <= MAX_ORIGINATING_ERROR_CHAIN_LENGTH
        && chain
            .iter()
            .all(|id| *id != HostEffectExecutorErrorId::RecoveryRequired)
        && chain
            .iter()
            .enumerate()
            .all(|(index, id)| !chain[..index].contains(id))
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct CommandCaptureDigest {
    pub(super) command_index: usize,
    pub(super) exit_code: i32,
    pub(super) stdout_sha256: String,
    pub(super) stderr_sha256: String,
    pub(super) combined_sha256: String,
}

#[derive(Debug)]
pub(super) struct CommandCapture {
    pub(super) exit_code: i32,
    pub(super) stdout: Vec<u8>,
    pub(super) stderr: Vec<u8>,
}

impl CommandCapture {
    pub(super) fn empty_failure() -> Self {
        Self {
            exit_code: -1,
            stdout: Vec::new(),
            stderr: Vec::new(),
        }
    }

    pub(super) fn digest(
        &self,
        command_index: usize,
    ) -> Result<CommandCaptureDigest, HostEffectExecutorFailure> {
        if self.stdout.len() > EXACT_OUTPUT_LIMIT_BYTES
            || self.stderr.len() > EXACT_OUTPUT_LIMIT_BYTES
        {
            return Err(HostEffectExecutorFailure::new(
                HostEffectExecutorErrorId::OutputOverflow,
            ));
        }
        #[derive(Serialize)]
        struct Combined<'a> {
            schema: &'static str,
            command_index: usize,
            exit_code: i32,
            stdout: &'a [u8],
            stderr: &'a [u8],
        }
        Ok(CommandCaptureDigest {
            command_index,
            exit_code: self.exit_code,
            stdout_sha256: digest_bytes(&self.stdout),
            stderr_sha256: digest_bytes(&self.stderr),
            combined_sha256: digest_json(&Combined {
                schema: "harness-ultragoal.host-command-capture.v1",
                command_index,
                exit_code: self.exit_code,
                stdout: &self.stdout,
                stderr: &self.stderr,
            })?,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct HostEffectExecutionPolicy {
    timeout_ms: u64,
    stdout_limit_bytes: usize,
    stderr_limit_bytes: usize,
    inherited_environment: bool,
    environment_entries: usize,
    environment_sha256: String,
    policy_sha256: String,
}

impl HostEffectExecutionPolicy {
    /// The accepted argv binding requires a cleared environment and a one MiB
    /// output ceiling. This constructor deliberately rejects every supplied
    /// environment entry instead of inventing unbound HOME, PATH, locale, or
    /// loader variables at the executor boundary.
    pub(in crate::distribution::host_effect) fn strict(
        timeout_ms: u64,
        environment: &[(String, String)],
    ) -> Result<Self, HostEffectExecutorFailure> {
        if !environment.is_empty() {
            return Err(HostEffectExecutorFailure::new(
                HostEffectExecutorErrorId::EnvironmentInjection,
            ));
        }
        if timeout_ms == 0 || timeout_ms > MAX_TIMEOUT_MS {
            return Err(HostEffectExecutorFailure::new(
                HostEffectExecutorErrorId::InvalidPolicy,
            ));
        }
        #[derive(Serialize)]
        struct EmptyEnvironment {
            schema: &'static str,
            inherited: bool,
            entries: usize,
        }
        let environment_sha256 = digest_json(&EmptyEnvironment {
            schema: "harness-ultragoal.cleared-host-environment.v1",
            inherited: false,
            entries: 0,
        })?;
        #[derive(Serialize)]
        struct Policy<'a> {
            schema: &'static str,
            timeout_ms: u64,
            stdout_limit_bytes: usize,
            stderr_limit_bytes: usize,
            inherited_environment: bool,
            environment_sha256: &'a str,
            shell: bool,
            process_group_containment: bool,
        }
        let policy_sha256 = digest_json(&Policy {
            schema: "harness-ultragoal.supported-host-execution-policy.v1",
            timeout_ms,
            stdout_limit_bytes: EXACT_OUTPUT_LIMIT_BYTES,
            stderr_limit_bytes: EXACT_OUTPUT_LIMIT_BYTES,
            inherited_environment: false,
            environment_sha256: &environment_sha256,
            shell: false,
            process_group_containment: true,
        })?;
        Ok(Self {
            timeout_ms,
            stdout_limit_bytes: EXACT_OUTPUT_LIMIT_BYTES,
            stderr_limit_bytes: EXACT_OUTPUT_LIMIT_BYTES,
            inherited_environment: false,
            environment_entries: 0,
            environment_sha256,
            policy_sha256,
        })
    }

    pub(super) const fn timeout(&self) -> Duration {
        Duration::from_millis(self.timeout_ms)
    }

    pub(super) const fn stdout_limit(&self) -> usize {
        self.stdout_limit_bytes
    }

    pub(super) const fn stderr_limit(&self) -> usize {
        self.stderr_limit_bytes
    }

    pub(crate) fn environment_sha256(&self) -> &str {
        &self.environment_sha256
    }

    pub(crate) fn policy_sha256(&self) -> &str {
        &self.policy_sha256
    }
}

#[derive(Clone, Default)]
pub(crate) struct HostEffectCancellation {
    cancelled: Arc<AtomicBool>,
}

impl HostEffectCancellation {
    pub(crate) fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub(crate) fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

pub(crate) struct HostEffectExecutionReceipt {
    effect_identity_sha256: String,
    outcome: HostEffectOutcome,
    terminal_ledger_head: HostEffectLedgerHead,
    command_output_sha256: Vec<String>,
    acknowledgement: PublicationAcknowledgementIdentity,
    acknowledgement_json: Vec<u8>,
    publication_classification: PublicationClassification,
}

impl std::fmt::Debug for HostEffectExecutionReceipt {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("HostEffectExecutionReceipt")
            .field("effect_identity_sha256", &self.effect_identity_sha256)
            .field("terminal_ledger_head", &self.terminal_ledger_head)
            .field("command_count", &self.command_output_sha256.len())
            .field(
                "publication_classification",
                &self.publication_classification.id(),
            )
            .finish_non_exhaustive()
    }
}

impl HostEffectExecutionReceipt {
    pub(super) fn new(
        effect_identity_sha256: String,
        outcome: HostEffectOutcome,
        terminal_ledger_head: HostEffectLedgerHead,
        command_output_sha256: Vec<String>,
        acknowledgement: PublicationAcknowledgementIdentity,
        acknowledgement_json: Vec<u8>,
        publication_classification: PublicationClassification,
    ) -> Self {
        Self {
            effect_identity_sha256,
            outcome,
            terminal_ledger_head,
            command_output_sha256,
            acknowledgement,
            acknowledgement_json,
            publication_classification,
        }
    }

    pub(crate) fn effect_identity_sha256(&self) -> &str {
        &self.effect_identity_sha256
    }

    pub(crate) fn outcome(&self) -> &HostEffectOutcome {
        &self.outcome
    }

    pub(crate) fn terminal_ledger_head(&self) -> &HostEffectLedgerHead {
        &self.terminal_ledger_head
    }

    pub(crate) fn command_output_sha256(&self) -> &[String] {
        &self.command_output_sha256
    }

    pub(crate) fn acknowledgement(&self) -> &PublicationAcknowledgementIdentity {
        &self.acknowledgement
    }

    pub(crate) fn acknowledgement_json(&self) -> &[u8] {
        &self.acknowledgement_json
    }

    pub(crate) fn publication_classification(&self) -> &PublicationClassification {
        &self.publication_classification
    }
}

pub(super) fn digest_json(value: &impl Serialize) -> Result<String, HostEffectExecutorFailure> {
    serde_json::to_vec(value)
        .map(|bytes| digest_bytes(&bytes))
        .map_err(|_| HostEffectExecutorFailure::new(HostEffectExecutorErrorId::Io))
}

pub(super) fn digest_bytes(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
