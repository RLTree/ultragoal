use std::collections::BTreeSet;

use super::context::{BuildRequest, LiveContext};
use super::reuse::common::{capture_guard, capture_receipt, issue_execution, observe_execution};
use super::routine_work::{
    CheckClass, DependencyResult, DirtySnapshot, ImpactGraph, LocalDirtyTree, PathMatcher,
    PlanMode, PlanRequest, PreparedRoutineExecution, ReportDisposition, ReportStatus,
    ReuseDecision, RoutineAdapterSpec, RoutineEffectRequest, RoutineError, RoutineErrorId,
    RoutineInvocationSpec, RoutineMediatedIntent, RoutineMediatedOutcome,
    RoutineMediationAuthority, RoutineMediationBatch, RoutinePlan, SkipReason, assess_reuse,
    begin_routine_mediation, bind_mediated_expectation, bind_mediated_witness,
    bind_routine_invocation, observe_mediated_incomplete, observe_mediated_outcome, plan_routine,
    prepare_routine_execution, reconcile_routine_execution, set_test_live_authority_hook,
};
use super::support::{TempRepo, fallback_graph, graph, node, path, route, sha};

struct AdapterFixture {
    repo: TempRepo,
    context: LiveContext,
    graph: ImpactGraph,
    snapshot: DirtySnapshot,
    plan: RoutinePlan,
}

fn adapter_context(repo: &TempRepo, profile: &str) -> LiveContext {
    LiveContext::build(
        BuildRequest::new(repo.root())
            .bind_non_secret_configuration("profile", profile)
            .probe_tool("sandbox-exec")
            .probe_tool("true"),
    )
    .unwrap()
}

fn adapter_graph() -> ImpactGraph {
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

fn dirty_fixture(label: &str) -> AdapterFixture {
    let repo = TempRepo::new(label);
    repo.write(".git/info/exclude", b"routine-cache/\n");
    repo.write("src/lib.rs", b"pub fn value() -> u8 { 9 }\n");
    let context = adapter_context(&repo, "routine");
    let snapshot = LocalDirtyTree::capture(&context).unwrap();
    let graph = adapter_graph();
    let plan = plan_routine(&context, &graph, &snapshot, PlanRequest::routine()).unwrap();
    assert_eq!(
        plan.checks()
            .iter()
            .map(|check| check.node_id())
            .collect::<Vec<_>>(),
        ["syntax", "compile", "unit"]
    );
    AdapterFixture {
        repo,
        context,
        graph,
        snapshot,
        plan,
    }
}

fn clean_fixture(label: &str) -> AdapterFixture {
    let repo = TempRepo::new(label);
    repo.write(".git/info/exclude", b"routine-cache/\n");
    let context = adapter_context(&repo, "routine");
    let snapshot = LocalDirtyTree::capture(&context).unwrap();
    let graph = adapter_graph();
    let plan = plan_routine(&context, &graph, &snapshot, PlanRequest::routine()).unwrap();
    AdapterFixture {
        repo,
        context,
        graph,
        snapshot,
        plan,
    }
}

fn invocation_specs(context: &LiveContext, plan: &RoutinePlan) -> Vec<RoutineInvocationSpec> {
    plan.checks()
        .iter()
        .map(|check| {
            bind_routine_invocation(
                context,
                plan,
                check.node_id(),
                vec![format!("--check={}", check.node_id())],
                60_000,
                4 * 1024 * 1024,
                vec![path("target/routine")],
            )
            .unwrap()
        })
        .collect()
}

fn request(fixture: &AdapterFixture) -> RoutineEffectRequest {
    let spec =
        RoutineAdapterSpec::new("routine", invocation_specs(&fixture.context, &fixture.plan));
    match prepare_routine_execution(
        &fixture.context,
        &fixture.graph,
        &fixture.snapshot,
        &fixture.plan,
        spec,
    )
    .unwrap()
    {
        PreparedRoutineExecution::Effect(request) => request,
        PreparedRoutineExecution::NoOp(_) => panic!("dirty plan emitted no-op"),
    }
}

fn preparation_error(result: Result<PreparedRoutineExecution, RoutineError>) -> RoutineError {
    match result {
        Ok(_) => panic!("invalid adapter preparation succeeded"),
        Err(error) => error,
    }
}

fn mediation(fixture: &AdapterFixture) -> (RoutineMediationAuthority, Vec<RoutineMediatedIntent>) {
    begin_routine_mediation(&fixture.context, &fixture.plan, request(fixture))
        .unwrap()
        .into_parts()
}

fn mediation_error(result: Result<RoutineMediationBatch, RoutineError>) -> RoutineError {
    match result {
        Ok(_) => panic!("invalid mediation transition succeeded"),
        Err(error) => error,
    }
}

fn observation_error(result: Result<RoutineMediatedOutcome, RoutineError>) -> RoutineError {
    match result {
        Ok(_) => panic!("invalid mediated observation succeeded"),
        Err(error) => error,
    }
}

fn skipped_outcomes(intents: Vec<RoutineMediatedIntent>) -> Vec<RoutineMediatedOutcome> {
    intents
        .into_iter()
        .map(|intent| {
            observe_mediated_incomplete(intent, ReportDisposition::Skipped(SkipReason::Cancelled))
                .unwrap()
        })
        .collect()
}

fn failed_outcomes(intents: Vec<RoutineMediatedIntent>) -> Vec<RoutineMediatedOutcome> {
    intents
        .into_iter()
        .map(|intent| {
            observe_mediated_incomplete(
                intent,
                ReportDisposition::Failed {
                    cause_code: "CHECK-FAILED".to_owned(),
                },
            )
            .unwrap()
        })
        .collect()
}

fn executed_outcomes(
    fixture: &AdapterFixture,
    intents: Vec<RoutineMediatedIntent>,
) -> Vec<RoutineMediatedOutcome> {
    let mut prior: Option<DependencyResult> = None;
    let mut outcomes = Vec::new();
    for intent in intents {
        let dependencies = prior.take().into_iter().collect();
        let expectation =
            bind_mediated_expectation(&fixture.context, &fixture.plan, &intent, dependencies)
                .unwrap();
        let execution = issue_execution(
            &fixture.repo,
            &fixture.context,
            expectation.expectation(),
            intent.intent().node_id().as_bytes(),
        );
        prior = Some(
            execution
                .work()
                .dependency_result(&fixture.context)
                .unwrap(),
        );
        let work = execution.into_parts().0;
        let witness =
            bind_mediated_witness(expectation, ReportDisposition::Executed(work)).unwrap();
        outcomes.push(observe_mediated_outcome(intent, witness).unwrap());
    }
    outcomes
}

#[test]
fn fixture_catalog_is_exact_and_has_no_claim_effect() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/routine-public-adapter/cases.json"
    ))
    .unwrap();
    assert_eq!(fixture["schema_version"], "RoutinePublicAdapterCases-v1");
    assert_eq!(fixture["claim_effect"], "none");
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 8);
    let ids = fixture["cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|case| case["id"].as_str().unwrap())
        .collect::<BTreeSet<_>>();
    assert_eq!(ids.len(), 8);
    assert!(
        fixture["cases"]
            .as_array()
            .unwrap()
            .iter()
            .all(|case| { case["process_spawns"] == 0 && case["workspace_writes"] == 0 })
    );
    assert!(
        fixture["no_claim_statement"]
            .as_str()
            .unwrap()
            .contains("do not execute workspace effects")
    );
}

#[test]
fn clean_noop_is_deterministic_exact_and_non_effectful() {
    let fixture = clean_fixture("adapter-clean-noop");
    assert_eq!(fixture.plan.affected_set().mode(), PlanMode::NoOp);
    let before_tree = fixture.repo.tree();
    let before_status = fixture.repo.status();
    let prepare = || {
        prepare_routine_execution(
            &fixture.context,
            &fixture.graph,
            &fixture.snapshot,
            &fixture.plan,
            RoutineAdapterSpec::new("routine", Vec::new()),
        )
        .unwrap()
    };
    let first = match prepare() {
        PreparedRoutineExecution::NoOp(projection) => projection,
        PreparedRoutineExecution::Effect(_) => panic!("clean no-op emitted an effect request"),
    };
    let second = match prepare() {
        PreparedRoutineExecution::NoOp(projection) => projection,
        PreparedRoutineExecution::Effect(_) => panic!("clean no-op emitted an effect request"),
    };
    assert_eq!(first.projection_id(), second.projection_id());
    assert_eq!(first.context_id(), fixture.context.context_id());
    assert_eq!(first.candidate_id(), fixture.plan.binding().candidate_id());
    assert_eq!(first.plan_id(), fixture.plan.plan_id());
    assert_eq!(first.result_scope(), "routine");
    assert!(first.selected().is_empty());
    assert_eq!(first.status(), ReportStatus::CompleteExecution);
    assert_eq!(first.effect_intent_count(), 0);
    assert!(first.support_limit().contains("no public command"));
    assert_eq!(fixture.repo.tree(), before_tree);
    assert_eq!(fixture.repo.status(), before_status);
}

#[test]
fn dirty_and_strict_preparation_bind_deterministic_complete_intents_without_effects() {
    let fixture = dirty_fixture("adapter-dirty-intents");
    let before_tree = fixture.repo.tree();
    let before_status = fixture.repo.status();
    let first = request(&fixture);
    let second = request(&fixture);
    assert_ne!(first.request_id(), second.request_id());
    assert_eq!(first.protocol_id(), second.protocol_id());
    assert_eq!(first.intents(), second.intents());
    assert_eq!(first.context_id(), fixture.context.context_id());
    assert_eq!(first.candidate_id(), fixture.plan.binding().candidate_id());
    assert_eq!(first.plan_id(), fixture.plan.plan_id());
    assert_eq!(first.result_scope(), "routine");
    assert!(std::mem::needs_drop::<RoutineEffectRequest>());
    for (order, intent) in first.intents().iter().enumerate() {
        let check = &fixture.plan.checks()[order];
        assert_eq!(intent.plan_order(), order);
        assert_eq!(intent.node_id(), check.node_id());
        assert_eq!(intent.selected_tool(), check.selected_tool());
        assert_eq!(
            intent.tool_identity_sha256(),
            check.selected_tool_identity()
        );
        assert!(intent.program_sha256().starts_with("sha256:"));
        assert!(!intent.program_path_hex().is_empty());
        assert!(intent.program_byte_length() > 0);
        assert_eq!(
            intent.program_unix_mode(),
            fixture
                .context
                .capabilities()
                .tool(check.selected_tool())
                .unwrap()
                .unix_mode
        );
        assert_eq!(intent.argv()[0], check.selected_tool());
        assert_eq!(
            intent.working_directory(),
            fixture.context.worktree_root().to_str().unwrap()
        );
        assert_eq!(intent.environment_policy(), "clear-all-no-inheritance-v1");
        assert_eq!(
            intent.mediation_preflight(),
            "revalidate-context-candidate-tool-executable-output-scopes-before-effect-v1"
        );
        assert_eq!(intent.timeout_ms(), 60_000);
        assert_eq!(intent.output_budget_bytes(), 4 * 1024 * 1024);
        assert_eq!(
            intent.declared_output_scopes()[0].as_str(),
            "target/routine"
        );
        assert_eq!(
            intent.expected_dependency_nodes(),
            check.depends_on().iter().cloned().collect::<Vec<_>>()
        );
        assert_eq!(intent.input_id(), check.input_id());
    }
    assert_eq!(fixture.repo.tree(), before_tree);
    assert_eq!(fixture.repo.status(), before_status);

    let repo = TempRepo::new("adapter-strict-intents");
    repo.write("release.json", b"{\"changed\":true}\n");
    let context = repo.context("strict");
    let snapshot = LocalDirtyTree::capture(&context).unwrap();
    let graph = graph();
    let plan = plan_routine(&context, &graph, &snapshot, PlanRequest::routine()).unwrap();
    assert_eq!(plan.affected_set().mode(), PlanMode::Strict);
    let spec = RoutineAdapterSpec::new("routine", invocation_specs(&context, &plan));
    let PreparedRoutineExecution::Effect(strict) =
        prepare_routine_execution(&context, &graph, &snapshot, &plan, spec).unwrap()
    else {
        panic!("strict plan emitted no-op")
    };
    assert_eq!(strict.intents().len(), plan.checks().len());
    assert_eq!(
        strict
            .intents()
            .iter()
            .map(|intent| intent.node_id())
            .collect::<Vec<_>>(),
        plan.checks()
            .iter()
            .map(|check| check.node_id())
            .collect::<Vec<_>>()
    );
}

#[test]
fn malformed_invocation_sets_and_stale_bindings_refuse_without_writes() {
    let fixture = dirty_fixture("adapter-invalid-preparation");
    let before_tree = fixture.repo.tree();
    let before_status = fixture.repo.status();

    let mut missing = invocation_specs(&fixture.context, &fixture.plan);
    missing.pop();
    assert_eq!(
        preparation_error(prepare_routine_execution(
            &fixture.context,
            &fixture.graph,
            &fixture.snapshot,
            &fixture.plan,
            RoutineAdapterSpec::new("routine", missing),
        ))
        .cause(),
        "adapter-invocation-cardinality-inexact"
    );

    let mut duplicate = invocation_specs(&fixture.context, &fixture.plan);
    duplicate[1] = bind_routine_invocation(
        &fixture.context,
        &fixture.plan,
        "syntax",
        vec!["--duplicate".to_owned()],
        60_000,
        1024,
        Vec::new(),
    )
    .unwrap();
    assert_eq!(
        preparation_error(prepare_routine_execution(
            &fixture.context,
            &fixture.graph,
            &fixture.snapshot,
            &fixture.plan,
            RoutineAdapterSpec::new("routine", duplicate),
        ))
        .cause(),
        "adapter-invocation-node-duplicated"
    );

    let mut unknown = invocation_specs(&fixture.context, &fixture.plan);
    let first = unknown.remove(0).test_with_node_id("unknown");
    unknown.insert(0, first);
    assert_eq!(
        preparation_error(prepare_routine_execution(
            &fixture.context,
            &fixture.graph,
            &fixture.snapshot,
            &fixture.plan,
            RoutineAdapterSpec::new("routine", unknown),
        ))
        .cause(),
        "adapter-invocation-node-unknown"
    );

    let mut reordered = invocation_specs(&fixture.context, &fixture.plan);
    reordered.swap(0, 1);
    assert_eq!(
        preparation_error(prepare_routine_execution(
            &fixture.context,
            &fixture.graph,
            &fixture.snapshot,
            &fixture.plan,
            RoutineAdapterSpec::new("routine", reordered),
        ))
        .cause(),
        "adapter-invocation-order-invalid"
    );

    let mut substituted = invocation_specs(&fixture.context, &fixture.plan);
    let first = substituted
        .remove(0)
        .test_with_tool_identity(format!("sha256:{}", "0".repeat(64)));
    substituted.insert(0, first);
    assert_eq!(
        preparation_error(prepare_routine_execution(
            &fixture.context,
            &fixture.graph,
            &fixture.snapshot,
            &fixture.plan,
            RoutineAdapterSpec::new("routine", substituted),
        ))
        .cause(),
        "adapter-runner-binding-mismatch"
    );

    let mut program_substitution = invocation_specs(&fixture.context, &fixture.plan);
    let first = program_substitution
        .remove(0)
        .test_with_program_sha256(format!("sha256:{}", "1".repeat(64)));
    program_substitution.insert(0, first);
    assert_eq!(
        preparation_error(prepare_routine_execution(
            &fixture.context,
            &fixture.graph,
            &fixture.snapshot,
            &fixture.plan,
            RoutineAdapterSpec::new("routine", program_substitution),
        ))
        .cause(),
        "adapter-runner-binding-mismatch"
    );

    let mut hostile_argv = invocation_specs(&fixture.context, &fixture.plan);
    let first = hostile_argv
        .remove(0)
        .test_with_arguments(vec!["line\nbreak".to_owned()]);
    hostile_argv.insert(0, first);
    assert_eq!(
        preparation_error(prepare_routine_execution(
            &fixture.context,
            &fixture.graph,
            &fixture.snapshot,
            &fixture.plan,
            RoutineAdapterSpec::new("routine", hostile_argv),
        ))
        .cause(),
        "adapter-argv-invalid"
    );

    assert_eq!(
        preparation_error(prepare_routine_execution(
            &fixture.context,
            &fixture.graph,
            &fixture.snapshot,
            &fixture.plan,
            RoutineAdapterSpec::new(
                "bad scope",
                invocation_specs(&fixture.context, &fixture.plan)
            ),
        ))
        .cause(),
        "adapter-result-scope-invalid"
    );

    let stale_context = adapter_context(&fixture.repo, "other-profile");
    assert_eq!(
        preparation_error(prepare_routine_execution(
            &stale_context,
            &fixture.graph,
            &fixture.snapshot,
            &fixture.plan,
            RoutineAdapterSpec::new("routine", invocation_specs(&fixture.context, &fixture.plan)),
        ))
        .id(),
        RoutineErrorId::ContextMismatch
    );

    let other = dirty_fixture("adapter-stale-snapshot");
    assert_eq!(
        preparation_error(prepare_routine_execution(
            &fixture.context,
            &fixture.graph,
            &other.snapshot,
            &fixture.plan,
            RoutineAdapterSpec::new("routine", invocation_specs(&fixture.context, &fixture.plan)),
        ))
        .cause(),
        "adapter-context-graph-snapshot-plan-mismatch"
    );
    assert_eq!(
        preparation_error(prepare_routine_execution(
            &fixture.context,
            &fallback_graph(),
            &fixture.snapshot,
            &fixture.plan,
            RoutineAdapterSpec::new("routine", invocation_specs(&fixture.context, &fixture.plan)),
        ))
        .cause(),
        "adapter-context-graph-snapshot-plan-mismatch"
    );
    assert_eq!(fixture.repo.tree(), before_tree);
    assert_eq!(fixture.repo.status(), before_status);
}

#[test]
fn missing_runner_identity_and_unbounded_execution_policy_refuse_without_writes() {
    let fixture = dirty_fixture("adapter-invalid-policy");
    let before_tree = fixture.repo.tree();
    let before_status = fixture.repo.status();

    let mut missing_identity = invocation_specs(&fixture.context, &fixture.plan);
    let first = missing_identity.remove(0).test_with_tool_identity("");
    missing_identity.insert(0, first);
    assert_eq!(
        preparation_error(prepare_routine_execution(
            &fixture.context,
            &fixture.graph,
            &fixture.snapshot,
            &fixture.plan,
            RoutineAdapterSpec::new("routine", missing_identity),
        ))
        .cause(),
        "adapter-runner-binding-mismatch"
    );

    let mut zero_timeout = invocation_specs(&fixture.context, &fixture.plan);
    let first = zero_timeout.remove(0).test_with_timeout_ms(0);
    zero_timeout.insert(0, first);
    assert_eq!(
        preparation_error(prepare_routine_execution(
            &fixture.context,
            &fixture.graph,
            &fixture.snapshot,
            &fixture.plan,
            RoutineAdapterSpec::new("routine", zero_timeout),
        ))
        .cause(),
        "adapter-timeout-invalid"
    );

    let mut zero_budget = invocation_specs(&fixture.context, &fixture.plan);
    let first = zero_budget.remove(0).test_with_output_budget_bytes(0);
    zero_budget.insert(0, first);
    assert_eq!(
        preparation_error(prepare_routine_execution(
            &fixture.context,
            &fixture.graph,
            &fixture.snapshot,
            &fixture.plan,
            RoutineAdapterSpec::new("routine", zero_budget),
        ))
        .cause(),
        "adapter-output-budget-invalid"
    );

    let mut duplicate_scope = invocation_specs(&fixture.context, &fixture.plan);
    let first = duplicate_scope
        .remove(0)
        .test_with_output_scopes(vec![path("target/routine"), path("TARGET/routine")]);
    duplicate_scope.insert(0, first);
    assert_eq!(
        preparation_error(prepare_routine_execution(
            &fixture.context,
            &fixture.graph,
            &fixture.snapshot,
            &fixture.plan,
            RoutineAdapterSpec::new("routine", duplicate_scope),
        ))
        .cause(),
        "adapter-output-scope-duplicated"
    );
    assert_eq!(fixture.repo.tree(), before_tree);
    assert_eq!(fixture.repo.status(), before_status);
}

#[test]
fn partial_duplicate_unknown_reordered_unobserved_and_cross_session_outcomes_fail_closed() {
    let fixture = dirty_fixture("adapter-invalid-outcomes");
    let before_tree = fixture.repo.tree();
    let before_status = fixture.repo.status();

    let (partial_authority, partial_intents) = mediation(&fixture);
    let mut partial = skipped_outcomes(partial_intents);
    partial.pop();
    assert_eq!(
        reconcile_routine_execution(&fixture.context, &fixture.plan, partial_authority, partial,)
            .unwrap_err()
            .cause(),
        "adapter-outcome-cardinality-inexact"
    );

    let (duplicate_authority, duplicate_intents) = mediation(&fixture);
    let first_id = duplicate_intents[0].intent().intent_id().to_owned();
    let first_node = duplicate_intents[0].intent().node_id().to_owned();
    let mut duplicate = skipped_outcomes(duplicate_intents);
    let duplicate_row = duplicate
        .remove(1)
        .test_with_intent(first_id, 1, first_node);
    duplicate.insert(1, duplicate_row);
    assert_eq!(
        reconcile_routine_execution(
            &fixture.context,
            &fixture.plan,
            duplicate_authority,
            duplicate,
        )
        .unwrap_err()
        .cause(),
        "adapter-outcome-duplicated"
    );

    let (unknown_authority, unknown_intents) = mediation(&fixture);
    let first_node = unknown_intents[0].intent().node_id().to_owned();
    let mut unknown = skipped_outcomes(unknown_intents);
    let unknown_row =
        unknown
            .remove(0)
            .test_with_intent(format!("sha256:{}", "f".repeat(64)), 0, first_node);
    unknown.insert(0, unknown_row);
    assert_eq!(
        reconcile_routine_execution(&fixture.context, &fixture.plan, unknown_authority, unknown,)
            .unwrap_err()
            .cause(),
        "adapter-outcome-unknown"
    );

    let (reordered_authority, reordered_intents) = mediation(&fixture);
    let mut reordered = skipped_outcomes(reordered_intents);
    reordered.swap(0, 1);
    assert_eq!(
        reconcile_routine_execution(
            &fixture.context,
            &fixture.plan,
            reordered_authority,
            reordered,
        )
        .unwrap_err()
        .cause(),
        "adapter-outcome-order-invalid"
    );

    let (unobserved_authority, unobserved_intents) = mediation(&fixture);
    let mut unobserved = skipped_outcomes(unobserved_intents);
    let unobserved_row = unobserved.remove(0).test_with_observed(false);
    unobserved.insert(0, unobserved_row);
    assert_eq!(
        reconcile_routine_execution(
            &fixture.context,
            &fixture.plan,
            unobserved_authority,
            unobserved,
        )
        .unwrap_err()
        .cause(),
        "adapter-outcome-unobserved"
    );

    let (first_authority, first_intents) = mediation(&fixture);
    let cross_session = skipped_outcomes(first_intents);
    let (second_authority, second_intents) = mediation(&fixture);
    assert_eq!(
        first_authority.protocol_id(),
        second_authority.protocol_id()
    );
    assert_ne!(first_authority.request_id(), second_authority.request_id());
    let second_request_id = second_authority.request_id().to_owned();
    let second_protocol_id = second_authority.protocol_id().to_owned();
    let copied_ids = cross_session
        .into_iter()
        .map(|outcome| {
            outcome.test_with_identity(second_request_id.clone(), second_protocol_id.clone())
        })
        .collect();
    assert_eq!(
        reconcile_routine_execution(
            &fixture.context,
            &fixture.plan,
            second_authority,
            copied_ids,
        )
        .unwrap_err()
        .cause(),
        "adapter-outcome-session-mismatch"
    );
    drop(second_intents);

    let (skipped_authority, skipped_intents) = mediation(&fixture);
    let skipped = skipped_outcomes(skipped_intents);
    assert_eq!(
        reconcile_routine_execution(&fixture.context, &fixture.plan, skipped_authority, skipped,)
            .unwrap_err()
            .cause(),
        "adapter-outcome-not-complete"
    );

    let (failed_authority, failed_intents) = mediation(&fixture);
    let failed = failed_outcomes(failed_intents);
    assert_eq!(
        reconcile_routine_execution(&fixture.context, &fixture.plan, failed_authority, failed,)
            .unwrap_err()
            .cause(),
        "adapter-outcome-not-complete"
    );

    let replayed = request(&fixture);
    replayed.test_mark_transitioned().unwrap();
    assert_eq!(
        mediation_error(begin_routine_mediation(
            &fixture.context,
            &fixture.plan,
            replayed,
        ))
        .cause(),
        "adapter-effect-request-replayed"
    );
    assert_eq!(fixture.repo.tree(), before_tree);
    assert_eq!(fixture.repo.status(), before_status);
}

#[cfg(target_os = "macos")]
#[test]
fn genuine_ordered_outcomes_delegate_to_exact_report_reconciliation() {
    let _capture = capture_guard();
    let fixture = dirty_fixture("adapter-complete");
    let (authority, intents) = mediation(&fixture);
    assert_eq!(authority.requested_result_scope(), "routine");
    let execution_result_scope = authority.execution_result_scope().to_owned();
    let outcomes = executed_outcomes(&fixture, intents);
    let report =
        reconcile_routine_execution(&fixture.context, &fixture.plan, authority, outcomes).unwrap();
    assert_eq!(report.status(), ReportStatus::CompleteExecution);
    assert_eq!(report.context_id(), fixture.context.context_id());
    assert_eq!(report.candidate_id(), fixture.plan.binding().candidate_id());
    assert_eq!(report.result_scope(), execution_result_scope);
    assert!(report.result_scope().starts_with("rma:"));
    assert_eq!(report.selected().len(), 3);
    assert!(report.reused().is_empty());
    assert!(report.support_limit().contains("no claim decision"));
}

#[cfg(target_os = "macos")]
#[test]
fn cross_request_witnesses_and_inconsistent_dependency_artifacts_fail_closed() {
    let _capture = capture_guard();
    let fixture = dirty_fixture("adapter-result-substitution");

    let (_executed_a_authority, mut executed_a_intents) = mediation(&fixture);
    let executed_a_intent = executed_a_intents.remove(0);
    let executed_a_expectation = bind_mediated_expectation(
        &fixture.context,
        &fixture.plan,
        &executed_a_intent,
        Vec::new(),
    )
    .unwrap();
    let executed_a = issue_execution(
        &fixture.repo,
        &fixture.context,
        executed_a_expectation.expectation(),
        b"executed-a",
    )
    .into_parts()
    .0;
    let executed_a_witness = bind_mediated_witness(
        executed_a_expectation,
        ReportDisposition::Executed(executed_a),
    )
    .unwrap();
    let (_executed_b_authority, mut executed_b_intents) = mediation(&fixture);
    assert_eq!(
        observation_error(observe_mediated_outcome(
            executed_b_intents.remove(0),
            executed_a_witness,
        ))
        .cause(),
        "adapter-outcome-witness-issuance-mismatch"
    );

    let (_reuse_a_authority, mut reuse_a_intents) = mediation(&fixture);
    let reuse_a_intent = reuse_a_intents.remove(0);
    let reuse_a_expectation =
        bind_mediated_expectation(&fixture.context, &fixture.plan, &reuse_a_intent, Vec::new())
            .unwrap();
    let reuse_a_execution = issue_execution(
        &fixture.repo,
        &fixture.context,
        reuse_a_expectation.expectation(),
        b"reuse-a",
    );
    let reuse_a_receipt = capture_receipt(
        &fixture.repo,
        &fixture.context,
        reuse_a_execution.receipt_json(),
        "adapter-reuse-a",
    );
    let reuse_a_observed = observe_execution(&fixture.context, reuse_a_expectation.expectation());
    let verified_reuse = match assess_reuse(
        &fixture.context,
        reuse_a_expectation.expectation(),
        &reuse_a_receipt,
        &reuse_a_observed,
    )
    .unwrap()
    {
        ReuseDecision::Hit(evidence) => evidence,
        ReuseDecision::Miss(reason) => panic!("expected verified reuse, got {reason:?}"),
    };
    let reuse_a_witness = bind_mediated_witness(
        reuse_a_expectation,
        ReportDisposition::Reused(verified_reuse),
    )
    .unwrap();
    let (_reuse_b_authority, mut reuse_b_intents) = mediation(&fixture);
    assert_eq!(
        observation_error(observe_mediated_outcome(
            reuse_b_intents.remove(0),
            reuse_a_witness,
        ))
        .cause(),
        "adapter-outcome-witness-issuance-mismatch"
    );

    let (authority, mut intents) = mediation(&fixture);
    let syntax_intent = intents.remove(0);
    let syntax_expectation =
        bind_mediated_expectation(&fixture.context, &fixture.plan, &syntax_intent, Vec::new())
            .unwrap();
    let syntax_a = issue_execution(
        &fixture.repo,
        &fixture.context,
        syntax_expectation.expectation(),
        b"syntax-a",
    );
    let syntax_b = issue_execution(
        &fixture.repo,
        &fixture.context,
        syntax_expectation.expectation(),
        b"syntax-b",
    );
    let syntax_b_dependency = syntax_b.work().dependency_result(&fixture.context).unwrap();
    let syntax_witness = bind_mediated_witness(
        syntax_expectation,
        ReportDisposition::Executed(syntax_a.into_parts().0),
    )
    .unwrap();
    let syntax_outcome = observe_mediated_outcome(syntax_intent, syntax_witness).unwrap();
    let compile_intent = intents.remove(0);
    let compile_expectation = bind_mediated_expectation(
        &fixture.context,
        &fixture.plan,
        &compile_intent,
        vec![syntax_b_dependency],
    )
    .unwrap();
    let compile_b = issue_execution(
        &fixture.repo,
        &fixture.context,
        compile_expectation.expectation(),
        b"compile-b",
    );
    let compile_b_dependency = compile_b
        .work()
        .dependency_result(&fixture.context)
        .unwrap();
    let compile_witness = bind_mediated_witness(
        compile_expectation,
        ReportDisposition::Executed(compile_b.into_parts().0),
    )
    .unwrap();
    let compile_outcome = observe_mediated_outcome(compile_intent, compile_witness).unwrap();
    let unit_intent = intents.remove(0);
    let unit_expectation = bind_mediated_expectation(
        &fixture.context,
        &fixture.plan,
        &unit_intent,
        vec![compile_b_dependency],
    )
    .unwrap();
    let unit_b = issue_execution(
        &fixture.repo,
        &fixture.context,
        unit_expectation.expectation(),
        b"unit-b",
    );
    let unit_witness = bind_mediated_witness(
        unit_expectation,
        ReportDisposition::Executed(unit_b.into_parts().0),
    )
    .unwrap();
    let outcomes = vec![
        syntax_outcome,
        compile_outcome,
        observe_mediated_outcome(unit_intent, unit_witness).unwrap(),
    ];
    let error = reconcile_routine_execution(&fixture.context, &fixture.plan, authority, outcomes)
        .unwrap_err();
    assert_eq!(error.id(), RoutineErrorId::InvalidRequest);
    assert_eq!(error.cause(), "report-dependency-result-chain-inconsistent");
}

#[cfg(target_os = "macos")]
#[test]
fn candidate_mutation_during_delegated_reconciliation_never_returns_complete() {
    let _capture = capture_guard();
    let fixture = dirty_fixture("adapter-live-mutation");
    let (authority, intents) = mediation(&fixture);
    let outcomes = executed_outcomes(&fixture, intents);
    let root = fixture.repo.root().to_path_buf();
    set_test_live_authority_hook(move || {
        std::fs::write(root.join("src/lib.rs"), b"mutated during adapter report\n").unwrap();
    });
    let error = reconcile_routine_execution(&fixture.context, &fixture.plan, authority, outcomes)
        .unwrap_err();
    assert_eq!(error.id(), RoutineErrorId::ConcurrentMutation);
}

#[test]
fn preparation_and_pre_report_refusal_source_has_no_effect_primitive() {
    let adapter = include_str!("../../src/routine_work/runtime_adapter.rs");
    let model = include_str!("../../src/routine_work/runtime_adapter/model.rs");
    for forbidden in [
        "std::process",
        "Command::new",
        "fs::write",
        "File::create",
        "OpenOptions",
        "set_len(",
        "remove_file",
        "remove_dir",
        "TcpStream",
    ] {
        assert!(
            !adapter.contains(forbidden) && !model.contains(forbidden),
            "adapter contains effect primitive {forbidden}"
        );
    }
    assert!(!model.contains("impl Clone for RoutineEffectRequest"));
    assert!(!model.contains("Deserialize for RoutineEffectRequest"));
    assert!(std::mem::needs_drop::<RoutineEffectRequest>());
    assert_eq!(sha(adapter.as_bytes()).len(), 71);
    assert_eq!(sha(model.as_bytes()).len(), 71);
}
