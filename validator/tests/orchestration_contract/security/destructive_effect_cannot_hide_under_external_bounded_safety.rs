#[test]
fn destructive_effect_cannot_hide_under_external_bounded_safety() {
    let mut value = lease();
    value.safety_class = SafetyClass::ExternalBounded;
    value.owned_scope.effects = [effect(EffectClass::Destructive, "cleanup")].into();
    let mut expanded = policy();
    expanded
        .allowed_effects
        .insert(effect(EffectClass::Destructive, "cleanup"));
    assert_eq!(
        value.validate(&expanded).unwrap_err(),
        OrchestrationError::RootOnlyScope
    );
}

#[test]
fn malformed_policy_identity_is_rejected_before_any_event() {
    let mut malformed = policy();
    malformed
        .allowed_semantic_prefixes
        .insert("secret\ncanary".to_owned());
    let (sink, _) = CountingSink::new();
    assert_eq!(
        Orchestrator::new(graph_one(), malformed, binding(), root(), bootstrap(), sink)
            .err()
            .unwrap(),
        OrchestrationError::InvalidIdentifier
    );
}

#[test]
fn bootstrap_completed_nodes_must_be_known_and_dependency_closed() {
    let graph = WorkGraph::derive(vec![
        package("node-a", &[], "node_a"),
        package("node-b", &["node-a"], "node_b"),
    ])
    .unwrap();
    let mut unknown = bootstrap();
    unknown
        .completed_nodes
        .insert("invented-node".to_owned(), digest('7'));
    let (sink, _) = CountingSink::new();
    assert_eq!(
        Orchestrator::new(graph.clone(), policy(), binding(), root(), unknown, sink,)
            .err()
            .unwrap(),
        OrchestrationError::UnknownNode
    );

    let mut open_dependency = bootstrap();
    open_dependency
        .completed_nodes
        .insert("node-b".to_owned(), digest('6'));
    let (sink, _) = CountingSink::new();
    assert_eq!(
        Orchestrator::new(graph, policy(), binding(), root(), open_dependency, sink,)
            .err()
            .unwrap(),
        OrchestrationError::InvalidTransition
    );
}
