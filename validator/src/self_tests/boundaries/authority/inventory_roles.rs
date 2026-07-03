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
        "package_inventory",
        "generated_artifact",
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
        }),
        "builder-contract docs must not be package evidence surfaces: {surfaces:?}"
    );
}
