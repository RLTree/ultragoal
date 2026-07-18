use super::orchestration_fixture::*;
use crate::orchestration::*;
use std::collections::BTreeSet;

#[test]
fn dependency_closed_fanout_selects_all_independent_children() {
    let graph = WorkGraph::derive(vec![
        package("root-node", &[], "root_node"),
        package("child-a", &["root-node"], "child_a"),
        package("child-b", &["root-node"], "child_b"),
    ])
    .unwrap();
    let tools = BTreeSet::from(["hct-context".to_owned()]);
    let first = graph
        .plan(&WorkProgress {
            available_tools: tools.clone(),
            ..WorkProgress::default()
        })
        .unwrap();
    assert_eq!(first.ready, ["root-node"]);

    let second = graph
        .plan(&WorkProgress {
            completed_nodes: BTreeSet::from(["root-node".to_owned()]),
            available_tools: tools,
            ..WorkProgress::default()
        })
        .unwrap();
    assert_eq!(second.ready, ["child-a", "child-b"]);
}

#[test]
fn scope_conflict_is_deferred_deterministically() {
    let mut second = package("node-b", &[], "node_b");
    second.owned_scope = scope("node_a");
    let graph = WorkGraph::derive(vec![package("node-a", &[], "node_a"), second]).unwrap();
    let plan = graph
        .plan(&WorkProgress {
            available_tools: BTreeSet::from(["hct-context".to_owned()]),
            ..WorkProgress::default()
        })
        .unwrap();
    assert_eq!(plan.ready, ["node-a"]);
    assert_eq!(plan.blocked["node-b"], PlanBlock::ScopeConflict);
}

#[test]
fn missing_tool_blocks_only_its_node() {
    let mut independent = package("independent", &[], "independent");
    independent.required_tools.clear();
    let graph =
        WorkGraph::derive(vec![package("needs-tool", &[], "needs_tool"), independent]).unwrap();
    let plan = graph.plan(&WorkProgress::default()).unwrap();
    assert_eq!(plan.ready, ["independent"]);
    assert_eq!(plan.blocked["needs-tool"], PlanBlock::RequiredToolMissing);
}

#[test]
fn unknown_dependency_fails_closed() {
    assert_eq!(
        WorkGraph::derive(vec![package("node-a", &["missing"], "node_a")]).unwrap_err(),
        OrchestrationError::UnknownNode
    );
}

#[test]
fn dependency_cycle_fails_closed() {
    assert_eq!(
        WorkGraph::derive(vec![
            package("node-a", &["node-b"], "node_a"),
            package("node-b", &["node-a"], "node_b"),
        ])
        .unwrap_err(),
        OrchestrationError::DependencyCycle
    );
}

#[test]
fn duplicate_node_fails_closed() {
    assert_eq!(
        WorkGraph::derive(vec![
            package("node-a", &[], "node_a"),
            package("node-a", &[], "node_b"),
        ])
        .unwrap_err(),
        OrchestrationError::DuplicateNode
    );
}

#[test]
fn completed_set_must_be_dependency_closed() {
    let graph = WorkGraph::derive(vec![
        package("node-a", &[], "node_a"),
        package("node-b", &["node-a"], "node_b"),
    ])
    .unwrap();
    assert!(
        !graph
            .dependency_closed(&BTreeSet::from(["node-b".to_owned()]))
            .unwrap()
    );
    assert!(
        graph
            .dependency_closed(&BTreeSet::from(["node-a".to_owned(), "node-b".to_owned(),]))
            .unwrap()
    );
}

#[test]
fn arbitrary_product_semantic_node_names_are_supported() {
    let graph = WorkGraph::derive(vec![package("repair-plugin-discovery", &[], "repair")]).unwrap();
    assert_eq!(
        graph.nodes().collect::<Vec<_>>(),
        ["repair-plugin-discovery"]
    );
}

#[test]
fn root_serialized_package_never_fans_out_with_other_work() {
    let mut root_owned = package("root-owned", &[], "root_owned");
    root_owned.safety_class = SafetyClass::RootSerialized;
    let graph = WorkGraph::derive(vec![package("parallel", &[], "parallel"), root_owned]).unwrap();
    let plan = graph
        .plan(&WorkProgress {
            available_tools: BTreeSet::from(["hct-context".to_owned()]),
            ..WorkProgress::default()
        })
        .unwrap();
    assert_eq!(plan.ready.len(), 1);
    assert!(
        plan.blocked
            .values()
            .any(|block| *block == PlanBlock::ScopeConflict)
    );
}

#[test]
fn engine_grants_and_starts_two_disjoint_node_workers() {
    let graph = WorkGraph::derive(vec![
        package("node-a", &[], "node_a"),
        package("node-b", &[], "node_b"),
    ])
    .unwrap();
    let (sink, _) = CountingSink::new();
    let mut engine =
        Orchestrator::new(graph, policy(), binding(), root(), bootstrap(), sink).unwrap();
    engine.grant_lease(1, lease()).unwrap();
    let mut second = lease_with_scope("lease-002", "node-b", "worker-b", scope("node_b"));
    second.issued_tick = 2;
    engine.grant_lease(2, second).unwrap();
    engine.start(3, "lease-001").unwrap();
    engine.start(4, "lease-002").unwrap();
    assert_eq!(engine.events().len(), 5);
    assert!(engine.plan().unwrap().ready.is_empty());
}
