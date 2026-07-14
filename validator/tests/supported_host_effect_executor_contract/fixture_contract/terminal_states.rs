fn assert_terminal_state_matrix(cases: &Cases) {
    assert_eq!(cases.terminal_state_matrix.len(), 23);
    let terminals = cases
        .terminal_state_matrix
        .iter()
        .map(|row| {
            (
                row.case.as_str(),
                row.command_started,
                row.publication_may_exist,
                row.state.as_str(),
                row.recovery_required,
            )
        })
        .collect::<Vec<_>>();
    for terminal in [
        (
            "started-backend-failure-terminal-transition-failed-before-mutation",
            true,
            false,
            "in-flight",
            true,
        ),
        (
            "publication-failure-terminal-commit-target-reobservation-unavailable",
            true,
            true,
            "ambiguous",
            true,
        ),
        (
            "committed-publication-terminal-commit-target-reobservation-unavailable",
            true,
            true,
            "settled",
            true,
        ),
        (
            "publication-failure-terminal-io-target-reobservation-unavailable",
            true,
            true,
            "in-flight",
            true,
        ),
        (
            "started-backend-failure-terminal-transition-committed-but-unverifiable",
            true,
            false,
            "ambiguous",
            true,
        ),
        (
            "started-backend-failure-terminal-record-mutation-rejected",
            true,
            false,
            "terminal-claim-withheld",
            true,
        ),
        (
            "failure-during-publication-terminal-ledger-transition-failed",
            true,
            true,
            "in-flight",
            true,
        ),
        (
            "publication-committed-terminal-ledger-transition-failed",
            true,
            true,
            "in-flight",
            true,
        ),
        (
            "publication-committed-terminal-transition-and-target-reobservation-failed",
            true,
            true,
            "terminal-claim-withheld",
            true,
        ),
        (
            "acknowledgement-invalid-after-settlement",
            true,
            true,
            "settled",
            true,
        ),
    ] {
        assert!(
            terminals.contains(&terminal),
            "missing terminal case {}",
            terminal.0
        );
    }
    for case in [
        "unrelated-permit-global-head-advance-with-still-in-flight-record",
        "repeated-unrelated-head-advances-after-terminal-commit",
        "unrelated-head-advance-record-substitution",
        "head-rollback-after-terminal-and-unrelated-advance",
        "observation-error-after-unrelated-head-advance",
        "post-reservation-wrong-permit-in-flight-observation",
        "post-reservation-wrong-permit-terminal-observation",
        "post-reservation-wrong-permit-rejected-observation",
        "post-reservation-right-record-wrong-head-observation",
    ] {
        assert!(terminals.contains(&(case, true, false, "terminal-claim-withheld", true)));
    }
}
