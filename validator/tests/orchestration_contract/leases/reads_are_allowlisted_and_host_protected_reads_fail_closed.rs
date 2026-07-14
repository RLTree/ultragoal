#[test]
fn reads_are_allowlisted_and_host_protected_reads_fail_closed() {
    let mut unknown = lease();
    unknown.read_paths = [path("secrets/input")].into();
    assert_eq!(
        unknown.validate(&policy()).unwrap_err(),
        OrchestrationError::UnknownScope
    );
    let mut protected = lease();
    protected.read_paths = [path(".codex-worktree/env.sh")].into();
    assert_eq!(
        protected.validate(&policy()).unwrap_err(),
        OrchestrationError::RootOnlyScope
    );
}

#[test]
fn one_node_cannot_have_two_active_leases() {
    let mut registry = LeaseRegistry::default();
    registry.grant(lease(), &policy(), &binding()).unwrap();
    assert_eq!(
        registry
            .grant(
                lease_with_scope("lease-002", "node-a", "worker-b", scope("node_b")),
                &policy(),
                &binding(),
            )
            .unwrap_err(),
        OrchestrationError::LeaseConflict
    );
}

#[test]
fn worker_cannot_request_root_serialized_safety() {
    let mut value = lease();
    value.safety_class = SafetyClass::RootSerialized;
    assert_eq!(
        value.validate(&policy()).unwrap_err(),
        OrchestrationError::RootOnlyScope
    );
}

#[test]
fn read_only_lease_cannot_hide_owned_scope() {
    let mut value = lease();
    value.safety_class = SafetyClass::ReadOnly;
    assert_eq!(
        value.validate(&policy()).unwrap_err(),
        OrchestrationError::InvalidLease
    );
}

#[test]
fn read_only_reconnaissance_lease_has_reads_but_no_mutation_authority() {
    let mut package = package("node-a", &[], "node_a");
    package.safety_class = SafetyClass::ReadOnly;
    package.owned_scope = OwnedScope::default();
    let graph = WorkGraph::derive(vec![package]).unwrap();
    let (sink, calls) = CountingSink::new();
    let mut engine =
        Orchestrator::new(graph, policy(), binding(), root(), bootstrap(), sink).unwrap();
    let mut read_only = lease();
    read_only.safety_class = SafetyClass::ReadOnly;
    read_only.owned_scope = OwnedScope::default();
    engine.grant_lease(1, read_only).unwrap();
    engine.start(2, "lease-001").unwrap();
    assert_eq!(calls.get(), 0);
    assert!(engine.plan().unwrap().ready.is_empty());
}

#[test]
fn stale_binding_and_duplicate_id_are_rejected() {
    let mut registry = LeaseRegistry::default();
    let mut stale = lease();
    stale.binding = Binding::new(&digest('e'), &digest('f')).unwrap();
    assert_eq!(
        registry.grant(stale, &policy(), &binding()).unwrap_err(),
        OrchestrationError::StaleBinding
    );
    registry.grant(lease(), &policy(), &binding()).unwrap();
    assert_eq!(
        registry.grant(lease(), &policy(), &binding()).unwrap_err(),
        OrchestrationError::InvalidLease
    );
}

#[test]
fn revoked_scope_can_be_reallocated() {
    let mut registry = LeaseRegistry::default();
    registry.grant(lease(), &policy(), &binding()).unwrap();
    registry.revoke("lease-001").unwrap();
    registry
        .grant(
            lease_with_scope("lease-002", "node-a", "worker-b", scope("node_a")),
            &policy(),
            &binding(),
        )
        .unwrap();
}
