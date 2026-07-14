impl HostEffectRecoveryHandoff {
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
}
