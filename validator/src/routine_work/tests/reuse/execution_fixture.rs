use std::collections::BTreeMap;
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use super::super::capture::{
    CapturedArtifact, CapturedRun, CommandSpec, PublicArtifact, capture_public_for_test,
};
use super::super::context::{BuildRequest, LiveContext};
use super::super::routine_work::{
    CapturedExecution, CheckClass, DependencyResult, ImpactGraph, LocalDirtyTree, ObservedResult,
    PathMatcher, PlanRequest, ReuseExpectation, ReuseReceipt, RoutinePlan, RunOutcome,
    capture_executed_result, observe_result_artifact,
};
use super::super::scenario::{TempRepo, node, path, route, sha};

const CACHE_ROOT: &str = "routine-cache";
static CAPTURE_SERIAL: RwLock<()> = RwLock::new(());

pub(crate) fn capture_guard() -> RwLockReadGuard<'static, ()> {
    CAPTURE_SERIAL
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

pub(crate) fn capture_race_guard() -> RwLockWriteGuard<'static, ()> {
    CAPTURE_SERIAL
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

pub(crate) fn authority_plan(repo: &TempRepo) -> (LiveContext, RoutinePlan) {
    repo.write(".git/info/exclude", format!("{CACHE_ROOT}/\n").as_bytes());
    repo.write("src/lib.rs", b"pub fn value() -> u8 { 7 }\n");
    let context = authority_context(repo);
    let snapshot = LocalDirtyTree::capture(&context).unwrap();
    let plan = super::super::routine_work::plan_routine(
        &context,
        &authority_graph(),
        &snapshot,
        PlanRequest::routine(),
    )
    .unwrap();
    (context, plan)
}

pub(crate) fn authority_context(repo: &TempRepo) -> LiveContext {
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

pub(crate) fn expectation(
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

pub(crate) fn result_bytes(
    context: &LiveContext,
    expectation: &ReuseExpectation,
    behavior: &[u8],
) -> Vec<u8> {
    result_bytes_with(
        context,
        expectation,
        RunOutcome::Passed,
        true,
        sha(behavior),
        BTreeMap::from([("artifact".to_owned(), sha(b"artifact"))]),
    )
}

pub(crate) fn result_bytes_with(
    context: &LiveContext,
    expectation: &ReuseExpectation,
    outcome: RunOutcome,
    behavior_observed: bool,
    behavior_sha256: String,
    output_digests: BTreeMap<String, String>,
) -> Vec<u8> {
    let executable = context.capabilities().tool("true").unwrap();
    expectation.result_artifact_fixture(
        format!(
            "sha256:{}",
            executable.executable_sha256.as_deref().unwrap()
        ),
        outcome,
        behavior_observed,
        behavior_sha256,
        output_digests,
    )
}

pub(crate) fn issue_execution(
    repo: &TempRepo,
    context: &LiveContext,
    expectation: &ReuseExpectation,
    behavior: &[u8],
) -> CapturedExecution {
    let bytes = result_bytes(context, expectation, behavior);
    let run = capture_run(repo, context, expectation, &bytes);
    capture_executed_result(context, expectation, &run).unwrap()
}

pub(crate) fn capture_run(
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

pub(crate) fn observe_execution(
    context: &LiveContext,
    expectation: &ReuseExpectation,
) -> ObservedResult {
    let relative = format!("{CACHE_ROOT}/{}.result.json", expectation.node_id());
    let artifact = capture_public_for_test(context, vec![PublicArtifact::new(relative)])
        .unwrap()
        .remove(0);
    observe_result_artifact(context, expectation, &artifact).unwrap()
}

pub(crate) fn observe_bytes(
    repo: &TempRepo,
    context: &LiveContext,
    expectation: &ReuseExpectation,
    bytes: &[u8],
) -> ObservedResult {
    let relative = format!("{CACHE_ROOT}/{}.result.json", expectation.node_id());
    repo.write(&relative, bytes);
    let artifact = capture_public_for_test(
        context,
        vec![PublicArtifact::with_digest(&relative, sha(bytes))],
    )
    .unwrap()
    .remove(0);
    observe_result_artifact(context, expectation, &artifact).unwrap()
}

pub(crate) fn capture_receipt(
    repo: &TempRepo,
    context: &LiveContext,
    bytes: &[u8],
    label: &str,
) -> ReuseReceipt {
    let artifact = capture_bytes(repo, context, bytes, label).unwrap();
    ReuseReceipt::from_captured(&artifact).unwrap()
}

pub(crate) fn capture_bytes(
    repo: &TempRepo,
    context: &LiveContext,
    bytes: &[u8],
    label: &str,
) -> Result<CapturedArtifact, String> {
    let relative = format!("{CACHE_ROOT}/{label}.receipt.json");
    repo.write(&relative, bytes);
    capture_public_for_test(
        context,
        vec![PublicArtifact::with_digest(&relative, sha(bytes))],
    )
    .map(|mut artifacts| artifacts.remove(0))
}

pub(crate) fn syntax_evidence(
    repo: &TempRepo,
    context: &LiveContext,
    plan: &RoutinePlan,
) -> (
    ReuseExpectation,
    CapturedExecution,
    ReuseReceipt,
    ObservedResult,
) {
    let expectation = expectation(context, plan, "syntax", Vec::new());
    let execution = issue_execution(repo, context, &expectation, b"syntax-behavior");
    let receipt = capture_receipt(repo, context, execution.receipt_json(), "syntax");
    let observed = observe_execution(context, &expectation);
    (expectation, execution, receipt, observed)
}
