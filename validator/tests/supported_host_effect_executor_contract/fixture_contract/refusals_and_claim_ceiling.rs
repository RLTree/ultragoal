fn assert_refusals_and_claim_ceiling(cases: &Cases) {
    let refusals = cases
        .required_refusal_cases
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    assert_eq!(refusals.len(), cases.required_refusal_cases.len());
    for required in [
        "replay-after-terminal-record",
        "ledger-rollback-or-stale-head",
        "target-path-swap",
        "executable-mutation-between-commands",
        "nonempty-environment",
        "stdout-overflow",
        "timeout-after-process-start",
        "symlink-object",
        "hardlinked-object",
        "rename-race",
        "directory-fsync-failure",
        "prepublication-terminal-transition-failure-before-mutation",
        "prepublication-terminal-transition-committed-but-unverifiable",
        "terminal-recovery-binding-mutation",
        "terminal-record-state-mutation",
        "terminal-ledger-transition-failure-after-publication",
        "terminal-ledger-transition-and-target-reobservation-failure-after-publication",
        "publication-failure-terminal-commit-target-reobservation-loss",
        "committed-publication-terminal-commit-target-reobservation-loss",
        "publication-failure-terminal-transition-io-target-reobservation-loss",
        "unrelated-permit-global-head-advance",
        "repeated-global-head-advance",
        "recovery-record-substitution",
        "recovery-head-rollback-staleness",
        "recovery-observation-error-after-head-advance",
        "post-reservation-wrong-permit-record",
        "post-reservation-right-record-wrong-head",
        "partial-acknowledgement",
        "false-pass-acknowledgement",
        "darwin-native-process-backend",
    ] {
        assert!(refusals.contains(required), "missing refusal {required}");
    }

    let source_contract = &cases.production_source_contract;
    assert!(source_contract.retained_executable_descriptor_only);
    assert!(source_contract.exact_accepted_argv_only);
    assert!(!source_contract.path_lookup_or_shell_fallback);
    assert!(source_contract.descriptor_relative_target_operations);
    assert!(source_contract.no_follow_and_exclusive_creation);
    assert!(source_contract.rename_no_replace);
    assert!(source_contract.file_and_directory_fsync);
    assert!(source_contract.publication_before_terminal_ledger_transition);
    assert!(source_contract.terminal_ledger_transition_before_acknowledgement);
    assert!(!source_contract.bare_boolean_acknowledgement);
    assert!(!source_contract.automatic_ambiguous_retry);

    let ceiling = &cases.claim_ceiling;
    assert!(ceiling.source_candidate);
    assert!(ceiling.macos_descriptor_relative_publication_unit_proof);
    assert!(!ceiling.macos_external_process_execution);
    assert!(!ceiling.linux_external_process_runtime_proof);
    assert!(!ceiling.freebsd_external_process_runtime_proof);
    assert!(!ceiling.public_route);
    assert!(!ceiling.installed_runtime);
    assert!(!ceiling.representative_real_host_journey);
    assert!(!ceiling.acceptance_or_release);
}
