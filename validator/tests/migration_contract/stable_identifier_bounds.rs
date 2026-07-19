#[test]
fn path_derived_stable_identifiers_have_a_separate_bounded_limit() {
    let at_limit = format!(
        "FIXTURE:{}",
        "a".repeat(crate::migration::MAX_STABLE_IDENTIFIER_BYTES - "FIXTURE:".len())
    );
    let accepted = MigrationInventory::new(
        sha('c'),
        sha('d'),
        sha('e'),
        sha('f'),
        vec![bounded_stable_id_surface(&at_limit)],
    );
    assert!(accepted.is_ok());

    let over_limit = format!("{at_limit}a");
    let rejected = MigrationInventory::new(
        sha('c'),
        sha('d'),
        sha('e'),
        sha('f'),
        vec![bounded_stable_id_surface(&over_limit)],
    )
    .expect_err("stable identifier beyond the derived bound must fail closed");
    assert_eq!(rejected.code(), "migration-surface-input-refused");
}

#[test]
fn current_long_path_derived_fixture_identifiers_are_valid() {
    for stable_id in [
        "FIXTURE:fixtures/red/product-strategy-positioning-research-eval-before-lane-planning-deep-research-v2-precedent-cited-without-required-planning-artifacts-red.json",
        "FIXTURE:fixtures/red/subagent-orchestration-explicitness-token-model-cost-result-reconciliation-subagent-launch-without-explicit-requested-contracted-role-red.json",
    ] {
        let inventory = MigrationInventory::new(
            sha('c'),
            sha('d'),
            sha('e'),
            sha('f'),
            vec![bounded_stable_id_surface(stable_id)],
        );
        assert!(inventory.is_ok(), "{stable_id}");
    }
}

fn bounded_stable_id_surface(stable_id: &str) -> InventorySurface {
    InventorySurface::observed(InventorySurfaceObservation {
        stable_id: stable_id.to_owned(),
        kind: "fixture".to_owned(),
        relative_path: "fixtures/red/stable-id-boundary.json".to_owned(),
        digest_sha256: sha('a'),
        file_kind: SurfaceFileKind::Regular,
        link_count: 1,
        status: SurfaceStatus::Active,
        active_readers: vec![],
        active_writers: vec![],
        public_routes: vec![],
        generated_outputs: vec![],
    })
}
