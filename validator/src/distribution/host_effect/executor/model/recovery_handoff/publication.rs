pub(super) struct TerminalTransitionRecoveryRequest {
    pub effect_identity_sha256: String,
    pub permit_id: String,
    pub ledger_head: HostEffectLedgerHead,
    pub ledger_record: HostEffectLedgerRecord,
    pub exact_current_ledger_observation: bool,
    pub outcome: HostEffectOutcome,
    pub originating_error_ids: Vec<HostEffectExecutorErrorId>,
    pub classification: HostEffectTerminalRecoveryClassification,
}

pub(super) struct PostReservationRecoveryRequest {
    pub effect_identity_sha256: String,
    pub permit_id: String,
    pub reservation_ledger_head: HostEffectLedgerHead,
    pub reservation_ledger_record: HostEffectLedgerRecord,
    pub ledger_head: HostEffectLedgerHead,
    pub ledger_record: HostEffectLedgerRecord,
    pub exact_current_ledger_observation: bool,
    pub publication_identity_sha256: Option<String>,
    pub prior_publication_observation: Option<PublicationInventoryObservation>,
    pub originating_error_ids: Vec<HostEffectExecutorErrorId>,
    pub ledger_classification: HostEffectPostReservationLedgerClassification,
    pub publication_classification: HostEffectPostReservationPublicationClassification,
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

    pub(super) fn terminal_transition(request: TerminalTransitionRecoveryRequest) -> Self {
        let TerminalTransitionRecoveryRequest {
            effect_identity_sha256,
            permit_id,
            ledger_head,
            ledger_record,
            exact_current_ledger_observation,
            outcome,
            originating_error_ids,
            classification,
        } = request;
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

    pub(super) fn post_reservation(request: PostReservationRecoveryRequest) -> Self {
        let PostReservationRecoveryRequest {
            effect_identity_sha256,
            permit_id,
            reservation_ledger_head,
            reservation_ledger_record,
            ledger_head,
            ledger_record,
            exact_current_ledger_observation,
            publication_identity_sha256,
            prior_publication_observation,
            originating_error_ids,
            ledger_classification,
            publication_classification,
        } = request;
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
}
