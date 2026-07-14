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
