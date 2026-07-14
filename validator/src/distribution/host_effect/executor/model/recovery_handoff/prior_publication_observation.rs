impl HostEffectRecoveryHandoff {
    pub(crate) fn prior_publication_observation(&self) -> Option<&PublicationInventoryObservation> {
        match self {
            Self::PostReservation {
                prior_publication_observation,
                ..
            } => prior_publication_observation.as_ref(),
            Self::Publication { .. } | Self::TerminalTransition { .. } => None,
        }
    }

    pub(crate) fn publication_identity_sha256(&self) -> Option<&str> {
        match self {
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
            Self::TerminalTransition { .. } | Self::PostReservation { .. } => None,
        }
    }

    pub(crate) fn terminal_classification(
        &self,
    ) -> Option<HostEffectTerminalRecoveryClassification> {
        match self {
            Self::Publication { .. } | Self::PostReservation { .. } => None,
            Self::TerminalTransition { classification, .. } => Some(*classification),
        }
    }

    pub(crate) fn ledger_record(&self) -> Option<&HostEffectLedgerRecord> {
        match self {
            Self::Publication { .. } => None,
            Self::TerminalTransition { ledger_record, .. } => Some(ledger_record),
            Self::PostReservation { ledger_record, .. } => Some(ledger_record),
        }
    }

    pub(crate) fn outcome(&self) -> Option<&HostEffectOutcome> {
        match self {
            Self::Publication { .. } | Self::PostReservation { .. } => None,
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
            Self::PostReservation {
                exact_current_publication_observation,
                ..
            } => *exact_current_publication_observation,
        }
    }

    pub(crate) fn binding_sha256(&self) -> Option<&str> {
        match self {
            Self::Publication { binding_sha256, .. }
            | Self::TerminalTransition { binding_sha256, .. }
            | Self::PostReservation { binding_sha256, .. } => Some(binding_sha256),
        }
    }
}
