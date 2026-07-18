use super::*;

#[test]
pub(crate) fn fixture_catalog_names_the_exact_below_root_journey_matrix() {
    let catalog = catalog();
    assert_eq!(
        catalog.schema_version,
        "RepositoryFitLiveJourneyFixtureCatalog-v1"
    );
    assert_eq!(catalog.temporary_root, BASE);
    assert_eq!(catalog.supported_host, "darwin");
    assert_eq!(catalog.claim_effect, "workspace_write");
    assert_eq!(
        catalog.production_permit_issuer,
        "darwin-owner-only-host-state"
    );
    assert_eq!(catalog.public_apply_dispatch, "successor-fit-apply");
    let observed = catalog
        .cases
        .iter()
        .map(|case| case.id.as_str())
        .collect::<BTreeSet<_>>();
    let expected = [
        "ambiguous-outcome",
        "complete-rollback",
        "conflict-refusal",
        "dirty-tree-preservation",
        "fifo-target-refusal",
        "fresh-setup",
        "inspect-command-zero-write",
        "link-and-case-alias-refusal",
        "no-effect-success-substitution",
        "partial-retrofit",
        "plan-command-zero-write",
        "public-authority-refusals",
        "public-binary-apply",
        "recovery-replan",
        "repeat-use-idempotence",
        "replay-refusal",
        "verify-as-apply-substitution",
        "verify-command-zero-write",
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    assert_eq!(observed, expected);
    assert_eq!(catalog.cases.len(), expected.len());
    assert!(catalog
        .cases
        .iter()
        .all(|case| !case.class.is_empty() && !case.expect.is_empty()));
    let read_ids = catalog
        .read_operations
        .iter()
        .map(|operation| operation.id.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        read_ids,
        [
            "inspect-command-zero-write",
            "plan-command-zero-write",
            "verify-command-zero-write",
        ]
        .into_iter()
        .collect()
    );
}

#[test]
pub(crate) fn direct_journeys_use_the_production_mediator_without_minting_root_authority() {
    let adapter = source("validator/src/repository_fit/product_adapter/mod.rs");
    let tests = source("validator/src/repository_fit/product_adapter/tests/mod.rs");
    let journeys =
        source("validator/src/repository_fit/product_adapter/tests/live_journeys/mod.rs");
    let protocol = source("validator/src/repository_fit/product_adapter/protocol/mod.rs");
    let permit = source("validator/src/repository_fit/product_adapter/root_permit/mod.rs");
    let local = source("validator/src/repository_fit/local/effects/mod.rs");
    assert!(tests.contains("mod live_journeys;"));
    assert!(journeys.contains("apply_with_root_permit"));
    assert!(journeys.contains("LocalEffects"));
    assert!(!journeys.contains("execute_for_test"));
    for name in [
        "positive_supported_host_fresh_setup_and_repeat_use_are_exact_and_idempotent",
        "positive_supported_host_partial_retrofit_preserves_dirty_user_state",
        "negative_conflict_and_settled_request_replay_refuse_without_effect",
        "mutation_failure_rolls_back_exactly_and_a_new_request_recovers",
        "race_after_effect_is_ambiguous_and_fresh_replanning_recovers",
        "security_link_and_case_alias_substitution_refuse_without_mutation",
        "special_file_fifo_target_refuses_before_effect_without_blocking",
        "false_pass_no_effect_success_and_verify_cannot_substitute_for_apply",
    ] {
        assert!(journeys.contains(&format!("fn {name}()")), "{name}");
    }
    assert!(protocol.contains("pub(crate) struct OpaqueFitApplyRequest"));
    assert!(permit.contains("pub(crate) trait RepositoryFitPermitEffects"));
    assert!(permit.contains("pub(crate) fn apply_with_root_permit"));
    assert!(permit.contains("#[cfg(test)]\npub(crate) struct TestRepositoryFitPermitAuthority"));
    assert!(local.contains("#[cfg(test)]\n        pub(crate) fn open_for_test"));
    assert!(local.contains("mutation_lease: false"));
    assert!(!adapter.contains("pub(crate) use root_permit"));
    assert!(!adapter.contains("pub use root_permit"));
    assert!(!adapter.contains("pub(crate) mod root_permit"));
}

#[test]
pub(crate) fn inspect_plan_and_verify_commands_are_recursive_tree_and_git_status_zero_write() {
    let catalog = catalog();
    let repository = CommandRepository::new();
    let initial = observe(&repository.root);
    assert!(!initial.git_status.is_empty(), "fixture must remain dirty");
    for operation in catalog.read_operations {
        let before = observe(&repository.root);
        let first = repository.run(&operation.args);
        let after_first = observe(&repository.root);
        assert_eq!(after_first, before, "{} hidden write", operation.id);
        assert_eq!(
            first.status.code(),
            Some(operation.exit_code),
            "{:?}",
            first
        );
        assert!(first.stderr.is_empty(), "{}: {:?}", operation.id, first);
        let value: serde_json::Value = serde_json::from_slice(&first.stdout).unwrap();
        assert_eq!(
            value["schema_version"], operation.schema_version,
            "{}",
            operation.id
        );
        let text = String::from_utf8_lossy(&first.stdout);
        assert!(!text.contains(PRIVATE_CANARY), "{}", operation.id);
        assert!(
            !text.contains(&*repository.root.to_string_lossy()),
            "{} exposed the target path",
            operation.id
        );

        let second = repository.run(&operation.args);
        let after_second = observe(&repository.root);
        assert_eq!(after_second, before, "{} repeat hidden write", operation.id);
        assert_eq!(second.status.code(), Some(operation.exit_code));
        assert_eq!(
            second.stdout, first.stdout,
            "{} was nondeterministic",
            operation.id
        );
        assert_eq!(
            second.stderr, first.stderr,
            "{} was nondeterministic",
            operation.id
        );
    }
    assert_eq!(observe(&repository.root), initial);
}
