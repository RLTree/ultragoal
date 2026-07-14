#[test]
fn post_reservation_reobservation_requires_coherent_head_and_recovering_permit_before_exact_classification()
 {
    let executor = source("validator/src/distribution/host_effect/executor.rs");
    let post_reservation =
        source_window(&executor, "fn reobserve_post_reservation", "#[cfg(test)]");
    assert!(post_reservation_reobservation_guard_is_complete(
        &post_reservation
    ));

    for (original, replacement) in [
        ("head_after != *record_after.current_head()", "false"),
        (
            "head_after != *record_after.current_head()",
            "head_before != *record_after.current_head()",
        ),
        (
            "head_after != *record_after.current_head()",
            "head_after != *record_before.current_head()",
        ),
        (
            "record_after.reservation().permit_id() != effect.permit().permit_id()",
            "false",
        ),
        (
            "record_after.reservation().permit_id() != effect.permit().permit_id()",
            "record_before.reservation().permit_id() != effect.permit().permit_id()",
        ),
        (
            "record_after.reservation().permit_id() != effect.permit().permit_id()",
            "effect.record().reservation().permit_id() != effect.permit().permit_id()",
        ),
    ] {
        let mutated = post_reservation.replacen(original, replacement, 1);
        assert!(
            !post_reservation_reobservation_guard_is_complete(&mutated),
            "post-reservation guard mutation was accepted: {replacement}"
        );
    }

    let permit_guard = "record_after.reservation().permit_id() != effect.permit().permit_id()";
    let without_guard = post_reservation.replacen(permit_guard, "false", 1);
    let moved_after_exact = without_guard.replacen(
        "classification: HostEffectPostReservationLedgerClassification::StillInFlight,",
        &format!(
            "classification: HostEffectPostReservationLedgerClassification::StillInFlight,\n                 // Mutant: guard moved after exact classification.\n                 if {permit_guard} {{ continue; }}"
        ),
        1,
    );
    assert!(
        !post_reservation_reobservation_guard_is_complete(&moved_after_exact),
        "a permit guard moved after exact StillInFlight classification was accepted"
    );
}

#[test]
fn post_reservation_boundary_enumerates_and_rejects_identity_free_returns() {
    let executor = source("validator/src/distribution/host_effect/executor.rs");
    let handoff = source_window(&executor, "fn execute_handoff", "fn execute_authorized");
    let guard = source_window(&executor, "fn execute_authorized", "fn execute_reserved");
    let publication_terminal = source_window(
        &executor,
        "fn finish_publication_failure",
        "fn committed_recovery_failure",
    );
    let backend_terminal = source_window(
        &executor,
        "fn finish_backend_failure",
        "fn finish_started_failure",
    );
    let timestamp_terminal = source_window(
        &executor,
        "fn finish_terminal_with_timestamp",
        "fn verified_terminal_failure",
    );
    let prepublication_terminal = source_window(
        &executor,
        "fn prepublication_terminal_recovery_failure",
        "fn identity_bearing_preflight_ledger_failure",
    );
    let committed_terminal = source_window(
        &executor,
        "fn committed_recovery_failure",
        "fn committed_before_terminal_recovery_failure",
    );
    let publication_transition = source_window(
        &executor,
        "fn publication_before_terminal_recovery_failure",
        "fn post_reservation_recovery_failure",
    );
    let test_entry = source_window(
        &executor,
        "fn execute_authorized_for_test",
        "fn receipt_name",
    );

    assert!(handoff.contains("post_reservation_recovery_failure"));
    assert!(guard.contains("match self.execute_reserved"));
    assert!(guard.contains("failure.recovery().is_none()"));
    assert!(guard.contains("post_reservation_recovery_failure"));
    assert_eq!(executor.matches("self.execute_reserved(").count(), 1);
    assert!(test_entry.contains("self.execute_authorized("));
    assert!(!test_entry.contains("self.execute_reserved("));

    let guarded_surfaces = [
        ("opaque-handoff-entry", handoff),
        ("post-reservation-result-guard", guard),
        ("backend-prepublication-terminal", backend_terminal),
        (
            "clock-and-target-prepublication-terminal",
            timestamp_terminal,
        ),
        ("prepublication-terminal-recovery", prepublication_terminal),
        (
            "publication-terminal-reobservation-loss",
            publication_terminal,
        ),
        ("committed-terminal-reobservation-loss", committed_terminal),
        (
            "publication-transition-reobservation-loss",
            publication_transition,
        ),
        ("unit-test-entry", test_entry),
    ];
    for (surface, source) in guarded_surfaces {
        for forbidden in [
            "HostEffectExecutorFailure::terminal",
            "return Err(HostEffectExecutorFailure::new",
            "Err(_) => HostEffectExecutorFailure::new",
            "Err(_) =>",
        ] {
            assert!(
                !source.contains(forbidden),
                "identity-free post-reservation return on {surface}: {forbidden}"
            );
        }
    }
    for specialized in [
        publication_terminal,
        committed_terminal,
        publication_transition,
    ] {
        assert!(specialized.contains("post_reservation_recovery_failure"));
        assert!(specialized.contains("observation_failure.id()"));
    }
    assert!(!executor.contains("HostEffectExecutorFailure::terminal"));

    for prepublication in [backend_terminal, timestamp_terminal] {
        assert!(prepublication.contains("Err(transition_failure)"));
        assert!(prepublication.contains("append_error_cause"));
        assert!(prepublication.contains("transition_failure.id()"));
        assert!(prepublication.contains("originating_error_ids"));
    }

    let model = source("validator/src/distribution/host_effect/executor/model.rs");
    assert!(model.contains("HostEffectRecoveryHandoff::PostReservation"));
    assert!(model.contains("host-effect-post-reservation-recovery.v1"));
    assert!(model.contains("originating_error_ids"));
    assert!(model.contains("host-effect-terminal-recovery.v2"));
    assert!(model.contains("valid_originating_error_chain"));
    assert!(model.contains("MAX_ORIGINATING_ERROR_CHAIN_LENGTH: usize = 4"));
    assert!(model.contains("HostEffectExecutorErrorId::RecoveryRequired"));
    assert!(model.contains("exact_current_ledger_observation"));
    assert!(model.contains("exact_current_publication_observation"));
    assert!(!model.contains("pub(super) const fn terminal("));
}
