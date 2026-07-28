use super::orchestration_fixture::*;
use crate::orchestration::*;

fn blocker(id: &str, node: &str, class: BlockerClass) -> Blocker {
    Blocker {
        blocker_id: id.to_owned(),
        node_id: node.to_owned(),
        class,
        evidence_digest: digest('7'),
    }
}

fn two_node_engine() -> Orchestrator<CountingSink> {
    let graph = WorkGraph::derive(vec![
        package("node-a", &[], "node_a"),
        package("node-b", &[], "node_b"),
    ])
    .unwrap();
    let (sink, _) = CountingSink::new();
    Orchestrator::new(graph, policy(), binding(), root(), bootstrap(), sink).unwrap()
}

#[test]
fn ordinary_failure_repairs_while_independent_work_advances() {
    let engine = two_node_engine();
    let plan = engine
        .recovery_plan(&[blocker(
            "failure-a",
            "node-a",
            BlockerClass::OrdinaryTechnical,
        )])
        .unwrap();
    assert_eq!(
        plan.directives["failure-a"].action,
        RecoveryAction::RetryOrRepair
    );
    assert!(!plan.directives["failure-a"].stop_dependent_work);
    assert_eq!(plan.advanceable_nodes, ["node-b"]);
}

#[test]
fn only_authority_access_and_destructive_classes_stop_dependent_work() {
    let engine = two_node_engine();
    let cases = [
        (
            BlockerClass::MissingDependency,
            RecoveryAction::ReplanDependencyClosed,
            false,
        ),
        (
            BlockerClass::MissingTool,
            RecoveryAction::ReplanDependencyClosed,
            false,
        ),
        (
            BlockerClass::ExternalAuthority,
            RecoveryAction::RequestExternalAuthority,
            true,
        ),
        (
            BlockerClass::RequiredAccessUnavailable,
            RecoveryAction::RequestRequiredAccess,
            true,
        ),
        (
            BlockerClass::DestructiveDecision,
            RecoveryAction::RequestDestructiveApproval,
            true,
        ),
    ];
    for (index, (class, action, stop)) in cases.into_iter().enumerate() {
        let id = format!("blocker-{index}");
        let plan = engine
            .recovery_plan(&[blocker(&id, "node-a", class)])
            .unwrap();
        assert_eq!(plan.directives[&id].action, action);
        assert_eq!(plan.directives[&id].stop_dependent_work, stop);
        assert_eq!(plan.advanceable_nodes, ["node-b"]);
    }
}

#[test]
fn duplicate_unknown_or_forged_blockers_fail_closed() {
    let engine = two_node_engine();
    let duplicate = blocker("same", "node-a", BlockerClass::OrdinaryTechnical);
    assert_eq!(
        engine
            .recovery_plan(&[duplicate.clone(), duplicate])
            .unwrap_err(),
        OrchestrationError::DuplicateOutput
    );
    assert_eq!(
        engine
            .recovery_plan(&[blocker(
                "unknown",
                "node-c",
                BlockerClass::OrdinaryTechnical,
            )])
            .unwrap_err(),
        OrchestrationError::UnknownNode
    );
    let mut forged = blocker("forged", "node-a", BlockerClass::ExternalAuthority);
    forged.evidence_digest = "not-a-digest".to_owned();
    assert_eq!(
        engine.recovery_plan(&[forged]).unwrap_err(),
        OrchestrationError::InvalidDigest
    );
}

#[test]
fn interrupted_root_must_recover_before_advancing() {
    let mut engine = two_node_engine();
    engine.interrupt_root(1).unwrap();
    let plan = engine.recovery_plan(&[]).unwrap();
    assert!(plan.root_recovery_required);
    assert!(plan.advanceable_nodes.is_empty());
}
