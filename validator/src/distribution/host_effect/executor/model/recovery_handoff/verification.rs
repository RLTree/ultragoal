impl HostEffectRecoveryHandoff {
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
}
