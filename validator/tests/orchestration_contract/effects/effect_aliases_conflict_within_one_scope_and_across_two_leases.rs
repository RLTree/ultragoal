#[test]
fn effect_aliases_conflict_within_one_scope_and_across_two_leases() {
    for class in [
        EffectClass::WorkspaceWrite,
        EffectClass::FixtureWrite,
        EffectClass::Network,
        EffectClass::ExternalWrite,
    ] {
        let lower = effect(class, "service/path");
        let mixed = effect(class, "Service/Path/Child");
        let mut duplicated = scope("node_a");
        duplicated.effects = [lower.clone(), mixed.clone()].into();
        assert_eq!(
            duplicated.validate().unwrap_err(),
            OrchestrationError::DuplicateOutput
        );

        let mut expanded = policy();
        expanded.allowed_effects.extend([lower, mixed]);
        let mut registry = LeaseRegistry::default();
        registry
            .grant(
                lease_with_scope(
                    "lease-001",
                    "node-a",
                    "worker-a",
                    scope_with_effect("node_a", class, "service/path"),
                ),
                &expanded,
                &binding(),
            )
            .unwrap();
        assert_eq!(
            registry
                .grant(
                    lease_with_scope(
                        "lease-002",
                        "node-b",
                        "worker-b",
                        scope_with_effect("node_b", class, "Service/Path/Child"),
                    ),
                    &expanded,
                    &binding(),
                )
                .unwrap_err(),
            OrchestrationError::LeaseConflict
        );
    }
}

#[test]
fn effect_authorization_stays_exact_and_non_aliases_remain_disjoint() {
    let mut exact = policy();
    exact
        .allowed_effects
        .insert(effect(EffectClass::WorkspaceWrite, "Service/Path"));
    assert_eq!(
        lease_with_scope(
            "lease-001",
            "node-a",
            "worker-a",
            scope_with_effect("node_a", EffectClass::WorkspaceWrite, "service/path"),
        )
        .validate(&exact)
        .unwrap_err(),
        OrchestrationError::UnknownScope
    );

    let mut expanded = policy();
    expanded.allowed_effects.extend([
        effect(EffectClass::WorkspaceWrite, "service/api"),
        effect(EffectClass::WorkspaceWrite, "service/apiv2"),
        effect(EffectClass::FixtureWrite, "service/api"),
    ]);
    let mut registry = LeaseRegistry::default();
    for (lease_id, node, owner, class, target) in [
        (
            "lease-001",
            "node-a",
            "worker-a",
            EffectClass::WorkspaceWrite,
            "service/api",
        ),
        (
            "lease-002",
            "node-b",
            "worker-b",
            EffectClass::WorkspaceWrite,
            "service/apiv2",
        ),
        (
            "lease-003",
            "node-c",
            "worker-c",
            EffectClass::FixtureWrite,
            "service/api",
        ),
    ] {
        registry
            .grant(
                lease_with_scope(
                    lease_id,
                    node,
                    owner,
                    scope_with_effect(node, class, target),
                ),
                &expanded,
                &binding(),
            )
            .unwrap();
    }
}
