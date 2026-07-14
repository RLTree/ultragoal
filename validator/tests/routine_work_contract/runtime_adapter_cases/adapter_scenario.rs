use super::*;

pub(crate) struct AdapterFixture {
    pub(crate) repo: TempRepo,
    pub(crate) context: LiveContext,
    pub(crate) graph: ImpactGraph,
    pub(crate) snapshot: DirtySnapshot,
    pub(crate) plan: RoutinePlan,
}

pub(crate) fn adapter_context(repo: &TempRepo, profile: &str) -> LiveContext {
    LiveContext::build(
        BuildRequest::new(repo.root())
            .bind_non_secret_configuration("profile", profile)
            .probe_tool("sandbox-exec")
            .probe_tool("true"),
    )
    .unwrap()
}

pub(crate) fn adapter_graph() -> ImpactGraph {
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

pub(crate) fn dirty_fixture(label: &str) -> AdapterFixture {
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

pub(crate) fn clean_fixture(label: &str) -> AdapterFixture {
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

pub(crate) fn invocation_specs(
    context: &LiveContext,
    plan: &RoutinePlan,
) -> Vec<RoutineInvocationSpec> {
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

pub(crate) fn request(fixture: &AdapterFixture) -> RoutineEffectRequest {
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

pub(crate) fn preparation_error(
    result: Result<PreparedRoutineExecution, RoutineError>,
) -> RoutineError {
    match result {
        Ok(_) => panic!("invalid adapter preparation succeeded"),
        Err(error) => error,
    }
}

pub(crate) fn mediation(
    fixture: &AdapterFixture,
) -> (RoutineMediationAuthority, Vec<RoutineMediatedIntent>) {
    begin_routine_mediation(&fixture.context, &fixture.plan, request(fixture))
        .unwrap()
        .into_parts()
}

pub(crate) fn mediation_error(result: Result<RoutineMediationBatch, RoutineError>) -> RoutineError {
    match result {
        Ok(_) => panic!("invalid mediation transition succeeded"),
        Err(error) => error,
    }
}

pub(crate) fn observation_error(
    result: Result<RoutineMediatedOutcome, RoutineError>,
) -> RoutineError {
    match result {
        Ok(_) => panic!("invalid mediated observation succeeded"),
        Err(error) => error,
    }
}

pub(crate) fn skipped_outcomes(intents: Vec<RoutineMediatedIntent>) -> Vec<RoutineMediatedOutcome> {
    intents
        .into_iter()
        .map(|intent| {
            observe_mediated_incomplete(intent, ReportDisposition::Skipped(SkipReason::Cancelled))
                .unwrap()
        })
        .collect()
}

pub(crate) fn failed_outcomes(intents: Vec<RoutineMediatedIntent>) -> Vec<RoutineMediatedOutcome> {
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

pub(crate) fn executed_outcomes(
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
pub(crate) fn fixture_catalog_is_exact_and_has_no_claim_effect() {
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
