use std::collections::BTreeMap;
use std::sync::{RwLock, RwLockReadGuard};

use super::base_support::{TempRepo, node, path, route, sha};
use super::capture::{CapturedRun, CommandSpec, PublicArtifact, capture_public_for_test};
use super::context::{BuildRequest, LiveContext};
use super::routine_work::{
    CapturedExecution, CheckClass, DependencyResult, ImpactGraph, LocalDirtyTree, ObservedResult,
    PathMatcher, PlanRequest, ReuseExpectation, ReuseReceipt, RoutinePlan, RunOutcome,
    capture_executed_result, observe_result_artifact,
};

const CACHE_ROOT: &str = "routine-cache";
static CAPTURE_SERIAL: RwLock<()> = RwLock::new(());

pub(super) fn capture_guard() -> RwLockReadGuard<'static, ()> {
    CAPTURE_SERIAL
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

pub(super) fn authority_plan(repo: &TempRepo) -> (LiveContext, RoutinePlan) {
    repo.write(".git/info/exclude", format!("{CACHE_ROOT}/\n").as_bytes());
    repo.write("src/lib.rs", b"pub fn value() -> u8 { 7 }\n");
    let context = authority_context(repo);
    let snapshot = LocalDirtyTree::capture(&context).unwrap();
    let plan = super::routine_work::plan_routine(
        &context,
        &authority_graph(),
        &snapshot,
        PlanRequest::routine(),
    )
    .unwrap();
    (context, plan)
}

pub(super) fn authority_context(repo: &TempRepo) -> LiveContext {
    LiveContext::build(
        BuildRequest::new(repo.root())
            .bind_non_secret_configuration("profile", "routine")
            .probe_tool("sandbox-exec")
            .probe_tool("true"),
    )
    .unwrap()
}

fn authority_graph() -> ImpactGraph {
    ImpactGraph::new(
        vec![
            node("syntax", &[], CheckClass::Routine, "true", None),
            node("compile", &["syntax"], CheckClass::Routine, "true", None),
            node("unit", &["compile"], CheckClass::Routine, "true", None),
        ],
        vec![route(
            "route-src",
            PathMatcher::Prefix(path("src")),
            &["compile"],
            false,
        )],
        Vec::new(),
    )
    .unwrap()
}

pub(super) fn expectation(
    context: &LiveContext,
    plan: &RoutinePlan,
    node_id: &str,
    dependencies: Vec<DependencyResult>,
) -> ReuseExpectation {
    ReuseExpectation::for_check(
        context,
        plan,
        plan.check(node_id).unwrap(),
        dependencies,
        "routine",
    )
    .unwrap()
}

pub(super) fn issue_execution(
    repo: &TempRepo,
    context: &LiveContext,
    expectation: &ReuseExpectation,
    behavior: &[u8],
) -> CapturedExecution {
    let bytes = result_bytes(context, expectation, behavior);
    let run = capture_run(repo, context, expectation, &bytes);
    capture_executed_result(context, expectation, &run).unwrap()
}

fn result_bytes(context: &LiveContext, expectation: &ReuseExpectation, behavior: &[u8]) -> Vec<u8> {
    let executable = context.capabilities().tool("true").unwrap();
    expectation.result_artifact_fixture(
        format!(
            "sha256:{}",
            executable.executable_sha256.as_deref().unwrap()
        ),
        RunOutcome::Passed,
        true,
        sha(behavior),
        BTreeMap::from([("artifact".to_owned(), sha(b"artifact"))]),
    )
}

fn capture_run(
    repo: &TempRepo,
    context: &LiveContext,
    expectation: &ReuseExpectation,
    bytes: &[u8],
) -> CapturedRun {
    let relative = format!("{CACHE_ROOT}/{}.result.json", expectation.node_id());
    repo.write(&relative, bytes);
    CommandSpec::catalog_read(expectation.node_id(), "true")
        .public_artifact(PublicArtifact::with_digest(&relative, sha(bytes)))
        .run(context)
        .unwrap()
}

pub(super) fn observe_execution(
    context: &LiveContext,
    expectation: &ReuseExpectation,
) -> ObservedResult {
    let relative = format!("{CACHE_ROOT}/{}.result.json", expectation.node_id());
    let artifact = capture_public_for_test(context, vec![PublicArtifact::new(relative)])
        .unwrap()
        .remove(0);
    observe_result_artifact(context, expectation, &artifact).unwrap()
}

pub(super) fn capture_receipt(
    repo: &TempRepo,
    context: &LiveContext,
    bytes: &[u8],
    label: &str,
) -> ReuseReceipt {
    let relative = format!("{CACHE_ROOT}/{label}.receipt.json");
    repo.write(&relative, bytes);
    let artifact = capture_public_for_test(
        context,
        vec![PublicArtifact::with_digest(&relative, sha(bytes))],
    )
    .unwrap()
    .remove(0);
    ReuseReceipt::from_captured(&artifact).unwrap()
}
