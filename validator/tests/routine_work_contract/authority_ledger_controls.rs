use super::routine_plan_fixture::RoutinePlanFixture;
use super::routine_work::{
    LocalDirtyTree, PlanRequest, RoutineAdapterSpec, RoutineCancellation, RoutineMediatorStatus,
    RoutineReuseInput, mediate_public_routine_execution, plan_routine, prepare_routine_execution,
};
use super::scenario::{TempRepo, graph};

#[test]
fn exact_noop_bypasses_authority_initialization_and_workspace_writes() {
    let mut repo = TempRepo::new("production-noop-zero-write");
    let context = repo.context("routine-noop");
    let snapshot = LocalDirtyTree::capture(&context).unwrap();
    let graph = graph();
    let plan = plan_routine(&context, &graph, &snapshot, PlanRequest::routine()).unwrap();
    assert!(plan.checks().is_empty());
    let prepared = prepare_routine_execution(
        &context,
        &graph,
        &snapshot,
        &plan,
        RoutineAdapterSpec::new("routine", Vec::new()),
    )
    .unwrap();
    let before = repo.tree();
    let result = mediate_public_routine_execution(
        None,
        &context,
        &plan,
        prepared,
        RoutineCancellation::new(),
        RoutineReuseInput::default(),
    )
    .unwrap();
    assert_eq!(result.status(), RoutineMediatorStatus::CompleteNoOp);
    assert_eq!(repo.tree(), before);
    repo.teardown_after_assertions();
}

#[test]
fn missing_authority_root_refuses_before_authority_initialization() {
    let mut fixture = RoutinePlanFixture::new("missing-authority-root-zero-write");
    let prepared = fixture.prepare().unwrap();
    let authority = fixture.repo.root().join("authority");
    let before = fixture.repo.tree();
    let error = mediate_public_routine_execution(
        None,
        &fixture.context,
        &fixture.plan,
        prepared,
        RoutineCancellation::new(),
        RoutineReuseInput::default(),
    )
    .unwrap_err();
    assert_eq!(error.cause(), "routine-production-authority-root-missing");
    assert!(!authority.exists());
    assert_eq!(fixture.repo.tree(), before);
    fixture.finish();
}
