#[test]
fn disjoint_leases_can_run_together() {
    let mut registry = LeaseRegistry::default();
    registry.grant(lease(), &policy(), &binding()).unwrap();
    let other = lease_with_scope("lease-002", "node-b", "worker-b", scope("node_b"));
    registry.grant(other, &policy(), &binding()).unwrap();
    assert_eq!(registry.values().count(), 2);
}

#[test]
fn path_ancestor_collision_is_rejected() {
    let mut registry = LeaseRegistry::default();
    let mut broad = scope("node_a");
    broad.paths = [path("validator/src/orchestration")].into();
    registry
        .grant(
            lease_with_scope("lease-001", "node-a", "worker-a", broad),
            &policy(),
            &binding(),
        )
        .unwrap();
    let error = registry
        .grant(
            lease_with_scope("lease-002", "node-b", "worker-b", scope("node_b")),
            &policy(),
            &binding(),
        )
        .unwrap_err();
    assert_eq!(error, OrchestrationError::LeaseConflict);
}

#[test]
fn semantic_generated_fixture_and_effect_collisions_are_rejected() {
    for mutate in 0..4 {
        let mut registry = LeaseRegistry::default();
        registry.grant(lease(), &policy(), &binding()).unwrap();
        let mut other_scope = scope("node_b");
        match mutate {
            0 => other_scope.semantic_symbols = ["orchestration::node_a::child".to_owned()].into(),
            1 => {
                other_scope.generated_outputs = [path("generated/orchestration/node_a.json")].into()
            }
            2 => {
                other_scope.fixtures =
                    [path("validator/tests/orchestration_contract/node_a.json")].into()
            }
            _ => {
                other_scope.effects =
                    [effect(EffectClass::WorkspaceWrite, "leased-source/node_a")].into()
            }
        }
        assert_eq!(
            registry
                .grant(
                    lease_with_scope("lease-002", "node-b", "worker-b", other_scope),
                    &policy(),
                    &binding(),
                )
                .unwrap_err(),
            OrchestrationError::LeaseConflict
        );
    }
}

#[test]
fn cross_category_filesystem_collision_is_rejected() {
    let mut registry = LeaseRegistry::default();
    registry.grant(lease(), &policy(), &binding()).unwrap();
    let mut other = scope("node_b");
    other.paths = [path("generated/orchestration/node_a.json")].into();
    let mut expanded = policy();
    expanded
        .allowed_paths
        .insert(path("generated/orchestration"));
    assert_eq!(
        registry
            .grant(
                lease_with_scope("lease-002", "node-b", "worker-b", other),
                &expanded,
                &binding(),
            )
            .unwrap_err(),
        OrchestrationError::LeaseConflict
    );
}

#[test]
fn unknown_and_root_only_scope_fail_closed() {
    let mut unknown = scope("node_a");
    unknown.paths = [path("outside/lease.rs")].into();
    assert_eq!(
        lease_with_scope("lease-001", "node-a", "worker-a", unknown)
            .validate(&policy())
            .unwrap_err(),
        OrchestrationError::UnknownScope
    );

    let mut root_only = scope("node_a");
    root_only.paths = [path("Cargo.lock")].into();
    assert_eq!(
        lease_with_scope("lease-001", "node-a", "worker-a", root_only)
            .validate(&policy())
            .unwrap_err(),
        OrchestrationError::RootOnlyScope
    );
}

#[test]
fn host_protected_scope_is_always_root_only() {
    let mut host = policy();
    host.allowed_paths.insert(path(".codex"));
    let mut owned = scope("node_a");
    owned.paths = [path(".codex/agents/writer.toml")].into();
    assert_eq!(
        lease_with_scope("lease-001", "node-a", "worker-a", owned)
            .validate(&host)
            .unwrap_err(),
        OrchestrationError::RootOnlyScope
    );
}

#[test]
fn generic_root_only_paths_are_denied_in_every_write_path_category() {
    for category in 0..3 {
        let root_only = path("plugin-manifest-draft.json");
        let mut expanded = policy();
        let location = match category {
            0 => {
                expanded.allowed_paths.insert(root_only.clone());
                "/owned_scope/paths"
            }
            1 => {
                expanded.allowed_generated_outputs.insert(root_only.clone());
                "/owned_scope/generated_outputs"
            }
            _ => {
                expanded.allowed_fixtures.insert(root_only.clone());
                "/owned_scope/fixtures"
            }
        };
        let mut encoded = serde_json::to_value(lease()).unwrap();
        *encoded.pointer_mut(location).unwrap() = serde_json::json!([root_only.as_str()]);
        let substituted: LeaseSpec = serde_json::from_value(encoded).unwrap();
        assert_eq!(
            substituted.validate(&expanded).unwrap_err(),
            OrchestrationError::RootOnlyScope
        );
    }
}

#[test]
fn host_protected_paths_are_denied_in_every_write_path_category() {
    for category in 0..3 {
        let protected = path(".codex/agents/writer.toml");
        let mut expanded = policy();
        let location = match category {
            0 => {
                expanded.allowed_paths.insert(protected.clone());
                "/owned_scope/paths"
            }
            1 => {
                expanded.allowed_generated_outputs.insert(protected.clone());
                "/owned_scope/generated_outputs"
            }
            _ => {
                expanded.allowed_fixtures.insert(protected.clone());
                "/owned_scope/fixtures"
            }
        };
        let mut encoded = serde_json::to_value(lease()).unwrap();
        *encoded.pointer_mut(location).unwrap() = serde_json::json!([protected.as_str()]);
        let substituted: LeaseSpec = serde_json::from_value(encoded).unwrap();
        assert_eq!(
            substituted.validate(&expanded).unwrap_err(),
            OrchestrationError::RootOnlyScope
        );
    }
}
