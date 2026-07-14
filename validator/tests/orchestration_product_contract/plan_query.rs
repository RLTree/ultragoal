use crate::orchestration::*;
use crate::orchestration_product::*;
use crate::product_fixture::*;
use std::collections::BTreeSet;

#[test]
fn clean_plan_and_query_are_recursive_zero_write_and_deterministic() {
    let (root, engine) = durable_engine("clean-plan", false);
    let head = engine.journal_head().unwrap().clone();
    drop(engine);
    let before = recursive_fingerprint(root.path());
    let workspace = ProductWorkspace::open(root.path()).unwrap();
    let query_request = QueryRequest {
        expected_head: head.clone(),
        tick: 1,
        live_workers: BTreeSet::new(),
    };
    let first = query(&context(), &workspace, &query_request).unwrap();
    let second = query(&context(), &workspace, &query_request).unwrap();
    assert_eq!(first, second);
    assert_eq!(first.plan.ready, vec!["node-a"]);
    assert!(first.recovery.ambiguous_operations.is_empty());

    let plan_request = PlanRequest {
        expected_head: head,
        tick: 1,
        live_workers: BTreeSet::new(),
        blockers: vec![],
    };
    let first_plan = plan(&context(), &workspace, &plan_request).unwrap();
    let second_plan = plan(&context(), &workspace, &plan_request).unwrap();
    assert_eq!(first_plan, second_plan);
    assert_eq!(
        first_plan.plan_id().unwrap(),
        second_plan.plan_id().unwrap()
    );
    assert_eq!(recursive_fingerprint(root.path()), before);
}

#[test]
fn blocker_plan_replans_ordinary_work_but_scopes_external_stops() {
    let (root, engine) = durable_engine("blocker-plan", false);
    let head = engine.journal_head().unwrap().clone();
    drop(engine);
    let workspace = ProductWorkspace::open(root.path()).unwrap();
    let request = PlanRequest {
        expected_head: head,
        tick: 1,
        live_workers: BTreeSet::new(),
        blockers: vec![Blocker {
            blocker_id: "missing-tool".to_owned(),
            node_id: "node-a".to_owned(),
            class: BlockerClass::MissingTool,
            evidence_digest: digest('7'),
        }],
    };
    let result = plan(&context(), &workspace, &request).unwrap();
    assert_eq!(
        result.recovery.directives["missing-tool"].action,
        RecoveryAction::ReplanDependencyClosed
    );
    assert!(!result.recovery.directives["missing-tool"].stop_dependent_work);
}

#[test]
fn unknown_live_worker_is_rejected_without_a_write() {
    let (root, engine) = durable_engine("unknown-worker", false);
    let head = engine.journal_head().unwrap().clone();
    drop(engine);
    let workspace = ProductWorkspace::open(root.path()).unwrap();
    let before = recursive_fingerprint(root.path());
    let error = query(
        &context(),
        &workspace,
        &QueryRequest {
            expected_head: head,
            tick: 1,
            live_workers: BTreeSet::from(["intruder-worker".to_owned()]),
        },
    )
    .unwrap_err();
    assert_eq!(error, ProductError::UnknownWorker);
    assert_eq!(recursive_fingerprint(root.path()), before);
}
