#[test]
fn foundational_inventory_declares_required_authority_roles() {
    let surfaces = crate::audit::law::authority_surfaces::required_surfaces_for_test();
    let roles = surfaces
        .iter()
        .map(|(role, _, _)| *role)
        .collect::<std::collections::BTreeSet<_>>();
    for required in [
        "source",
        "schema",
        "valid_fixture",
        "red_fixture_catalog",
        "package_manifest",
        "package_inventory",
        "setup_retrofit_output",
        "receipt",
        "standards",
        "source_obligation",
        "foundational_trace",
        "claim_guard",
        "final_packet_blocker",
        "update_goal_blocker",
    ] {
        assert!(
            roles.contains(required),
            "foundational law inventory is missing role {required}: {roles:?}"
        );
    }
    assert!(
        surfaces
            .iter()
            .any(|(role, path, packaged)| *role == "receipt"
                && *path == "validation_artifacts/coverage/coverage-receipt.json"
                && !*packaged),
        "runtime receipts may be required as live surfaces without becoming package inventory"
    );
    assert!(
        surfaces.iter().all(|(_, path, _)| {
            !path.starts_with("docs/ultragoal-contract-2026-07/")
                && !path.contains("parent-session-full-ultragoal")
                && *path != "docs/generated/observability/command-inventory.json"
        }),
        "builder-contract docs and the deauthorized command inventory must not be package evidence surfaces: {surfaces:?}"
    );
}

#[test]
fn foundational_inventory_rejects_missing_and_unpackaged_required_surfaces() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("authority-required-surfaces");
    std::fs::create_dir_all(&root).expect("required surfaces fixture root");
    let failures = crate::audit::law::authority_surfaces::required_surface_failures_for_test(
        &root,
        &std::collections::BTreeSet::new(),
    );
    assert!(
        failures
            .iter()
            .any(|(_, failure)| failure.contains("authority_surface_missing:role=source")),
        "{failures:?}"
    );
    assert!(
        failures.iter().any(|(_, failure)| {
            failure.contains("authority_surface_not_in_package_inventory:role=source")
        }),
        "{failures:?}"
    );
    assert!(
        !failures.iter().any(|(_, failure)| {
            failure.contains(
                "authority_surface_not_in_package_inventory:role=receipt:path=validation_artifacts/coverage/coverage-receipt.json"
            )
        }),
        "runtime coverage receipt is required at proof time but must not become package inventory: {failures:?}"
    );
    for rel in [
        "validation_artifacts/coverage/coverage-receipt.json",
        "validation_artifacts/ultragoal-audit/validator-receipt.json",
        "validation_artifacts/ultragoal-audit/red-fixture-report.json",
        "validation_artifacts/review/final-packet-proof.json",
        "validation_artifacts/cli/update-goal-eligibility.json",
    ] {
        assert!(
            !failures.iter().any(|(_, failure)| failure
                .contains(&format!("authority_surface_missing:role="))
                && failure.contains(rel)),
            "runtime materialization {rel} must remain a proof-time expectation, not a source-local inventory existence requirement: {failures:?}"
        );
    }
    std::fs::remove_dir_all(root).expect("cleanup required authority surfaces");
}
