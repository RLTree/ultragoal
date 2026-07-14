#[test]
fn executor_orders_command_publication_ledger_and_acknowledgement() {
    let executor = source("validator/src/distribution/host_effect/executor.rs");
    for required in [
        "DescriptorExecutionHandoff",
        "with_retained_authority",
        "has_current_in_flight_record",
        "self.backend.execute",
        "self.target.prepare",
        ".publish(prepared",
        "HostEffectState::Settled",
        "PublicationAcknowledgementIdentity::new",
        "from_canonical_json",
        ".acknowledge(",
        "PublicationClassificationId::AcknowledgedCommitted",
        "committed_before_terminal_recovery_failure",
        "prepublication_terminal_recovery_failure",
        "reobserve_terminal_failure",
        "HostEffectTerminalRecoveryClassification::StillInFlight",
        "HostEffectTerminalRecoveryClassification::TerminalCommittedAndVerified",
        "HostEffectTerminalRecoveryClassification::TerminalCommittedButUnverifiable",
        "post_reservation_recovery_failure",
    ] {
        assert!(
            executor.contains(required),
            "missing executor token {required}"
        );
    }
    let execution = source("validator/src/distribution/host_effect/executor/reservation/execution.rs");
    assert_before(
        &execution,
        "self.execute_reserved_commands",
        "self.publish_reserved(",
    );
    let publication = source("validator/src/distribution/host_effect/executor/publication/write.rs");
    assert_before(
        &publication,
        "let prepared_publication = match self.target.prepare(",
        ".publish(prepared",
    );
    assert_before(
        &publication,
        ".publish(prepared",
        "self.acknowledge_reserved_publication",
    );
    let acknowledgement =
        source("validator/src/distribution/host_effect/executor/publication/acknowledgement.rs");
    assert_before(
        &acknowledgement,
        "let terminal = self",
        "PublicationAcknowledgementIdentity::new",
    );
    assert_before(
        &acknowledgement,
        "PublicationAcknowledgementIdentity::new",
        ".acknowledge(",
    );
    assert!(!executor.contains("pub fn "));
    assert!(!executor.contains("std::process::Command"));
    assert!(!executor.contains("self.transition_terminal(effect, state, outcome_sha256)?;"));

    let model = source("validator/src/distribution/host_effect/executor/model.rs");
    for required in [
        "HostEffectRecoveryHandoff::TerminalTransition",
        "effect_identity_sha256",
        "permit_id",
        "ledger_head",
        "ledger_record",
        "exact_current_ledger_observation",
        "outcome",
        "originating_error_ids",
        "classification",
        "binding_sha256",
        "verify_binding",
        "HostEffectRecoveryHandoff::PostPublicationTerminalTransition",
        "HostEffectRecoveryHandoff::PostReservation",
        "prior_publication_observation",
        "exact_current_publication_observation",
        "CommittedBeforeTerminalTransitionObservationUnavailable",
        "HostEffectPostReservationLedgerClassification",
        "HostEffectPostReservationPublicationClassification",
        "originating_error_ids",
    ] {
        assert!(
            model.contains(required),
            "missing recovery model token {required}"
        );
    }

    let tests = source("validator/src/distribution/host_effect/executor/tests.rs");
    for required in [
        "positive_actual_publication_fsync_rename_ledger_and_ack_transaction_settles",
        "negative_started_backend_terminal_failure_before_mutation_returns_exact_recovery",
        "negative_clock_failure_terminal_transition_also_returns_exact_recovery",
        "negative_preflight_ledger_substitution_after_handoff_is_identity_bearing",
        "race_committed_terminal_error_is_reobserved_as_committed_but_unverifiable",
        "security_mutated_terminal_record_is_rejected_without_false_terminal_claim",
        "mutation_of_recovery_permit_binding_is_detected_as_false_pass",
        "mutation_of_recovery_error_chain_order_length_and_members_is_detected",
        "dual_failure_after_committed_publication_retains_identity_when_reobservation_is_unavailable",
        "publication_failure_terminal_commit_then_reobservation_loss_retains_identity",
        "committed_publication_terminal_commit_then_reobservation_loss_retains_identity",
        "publication_failure_terminal_io_then_reobservation_loss_retains_identity",
        "race_unrelated_permit_global_head_advance_with_still_in_flight_record_is_unavailable_and_bound",
        "race_repeated_unrelated_head_advances_after_terminal_commit_withhold_terminal_classification",
        "negative_unrelated_head_advance_record_substitution_is_unavailable_and_bound",
        "negative_head_rollback_after_terminal_and_unrelated_advance_is_unavailable_and_bound",
        "negative_observation_error_after_unrelated_head_advance_is_unavailable_and_bound",
        "wrong_permit_post_reservation_still_in_flight_observation_falls_back_bound_unavailable",
        "wrong_permit_post_reservation_terminal_observation_falls_back_bound_unavailable",
        "wrong_permit_post_reservation_rejected_observation_falls_back_bound_unavailable",
        "post_reservation_right_record_wrong_head_falls_back_bound_unavailable",
        "post_reservation_correct_permit_still_in_flight_positive_control_is_exact_and_bound",
        "post_reservation_correct_permit_terminal_positive_control_is_exact_and_bound",
        "post_reservation_correct_permit_rejected_positive_control_is_exact_and_bound",
        "post_reservation_observation_error_falls_back_bound_unavailable_and_preserves_chain",
        "wrong_permit_post_reservation_replay_preflight_route_falls_back_bound_unavailable",
        "FailTerminalLedger",
        "CommitThenFailTerminalLedger",
        "DisplaceTargetAndFailTerminalLedger",
        "CommitThenDisplaceTargetLedger",
        "AdvanceUnrelatedPermitThenFailTerminalLedger",
    ] {
        assert!(
            tests.contains(required),
            "missing named recovery test {required}"
        );
    }

    let dual_failure_start = executor
        .find("fn committed_before_terminal_recovery_failure")
        .unwrap();
    let dual_failure_end = executor[dual_failure_start..]
        .find("fn publication_before_terminal_recovery_failure")
        .map(|offset| dual_failure_start + offset)
        .unwrap();
    let dual_failure = &executor[dual_failure_start..dual_failure_end];
    assert!(
        dual_failure.contains("post_publication_terminal_transition_observation_unavailable")
    );
    assert!(dual_failure.contains("committed.observation.clone()"));
    assert!(dual_failure.contains("originating_error_ids"));
    assert!(dual_failure.contains("observation_failure.id()"));
    assert!(!dual_failure.contains("post_reservation_recovery_failure"));
    assert!(!dual_failure.contains("HostEffectExecutorFailure::new"));
    assert!(!dual_failure.contains("HostEffectExecutorFailure::terminal"));
}

#[test]
fn terminal_reobservation_requires_coherent_global_head_before_exact_classification() {
    let executor = source("validator/src/distribution/host_effect/executor.rs");
    assert!(terminal_reobservation_guard_is_complete(&executor));

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
    ] {
        let mutated = executor.replacen(original, replacement, 1);
        assert!(
            !terminal_reobservation_guard_is_complete(&mutated),
            "guard mutation was accepted: {replacement}"
        );
    }
}
