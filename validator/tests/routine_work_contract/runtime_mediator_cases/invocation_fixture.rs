use super::*;

pub(crate) fn invocations_with_read_source(
    fixture: &MediatorFixture,
    arguments: impl Fn(&str) -> Vec<String>,
    read_source: &str,
    timeout_ms: u64,
    output_budget_bytes: u64,
) -> Vec<RoutineInvocationSpec> {
    fixture
        .plan
        .checks()
        .iter()
        .map(|check| {
            bind_routine_invocation_with_read_sources(
                &fixture.context,
                &fixture.plan,
                check.node_id(),
                arguments(check.node_id()),
                vec![path(read_source)],
                timeout_ms,
                output_budget_bytes,
                vec![path(&format!("target/routine/{}", check.node_id()))],
            )
            .unwrap()
        })
        .collect()
}

pub(crate) fn prepared_with(
    fixture: &MediatorFixture,
    script: impl Fn(&str) -> String,
    timeout_ms: u64,
    output_budget_bytes: u64,
) -> PreparedRoutineExecution {
    prepared_with_arguments(
        fixture,
        |node_id| vec!["-c".to_owned(), script(node_id)],
        timeout_ms,
        output_budget_bytes,
    )
}

pub(crate) fn prepared_with_arguments(
    fixture: &MediatorFixture,
    arguments: impl Fn(&str) -> Vec<String>,
    timeout_ms: u64,
    output_budget_bytes: u64,
) -> PreparedRoutineExecution {
    prepare_routine_execution(
        &fixture.context,
        &fixture.graph,
        &fixture.snapshot,
        &fixture.plan,
        RoutineAdapterSpec::new(
            "routine",
            invocations_with_arguments(fixture, arguments, timeout_ms, output_budget_bytes),
        ),
    )
    .unwrap()
}

pub(crate) fn prepared_with_read_source(
    fixture: &MediatorFixture,
    arguments: impl Fn(&str) -> Vec<String>,
    read_source: &str,
    timeout_ms: u64,
    output_budget_bytes: u64,
) -> PreparedRoutineExecution {
    prepare_routine_execution(
        &fixture.context,
        &fixture.graph,
        &fixture.snapshot,
        &fixture.plan,
        RoutineAdapterSpec::new(
            "routine",
            invocations_with_read_source(
                fixture,
                arguments,
                read_source,
                timeout_ms,
                output_budget_bytes,
            ),
        ),
    )
    .unwrap()
}

pub(crate) fn prepared(fixture: &MediatorFixture) -> PreparedRoutineExecution {
    prepared_with(fixture, command_script, 10_000, 1024 * 1024)
}

pub(crate) fn effect_request(prepared: &PreparedRoutineExecution) -> &RoutineEffectRequest {
    match prepared {
        PreparedRoutineExecution::Effect(request) => request,
        PreparedRoutineExecution::NoOp(_) => panic!("dirty fixture emitted no-op"),
    }
}

pub(crate) fn issue_grant(
    prepared: &PreparedRoutineExecution,
    session_id: &str,
    recovery_for: Option<String>,
) -> RoutineRootGrant {
    RoutineRootGrant::test_issue(effect_request(prepared), session_id, recovery_for)
}

pub(crate) fn mediate(
    fixture: &MediatorFixture,
    prepared: PreparedRoutineExecution,
    grant: Option<RoutineRootGrant>,
    cancellation: RoutineCancellation,
    reuse: Vec<Vec<u8>>,
) -> RoutineMediationResult {
    mediate_prepared_routine_execution(
        &fixture.context,
        &fixture.plan,
        prepared,
        grant,
        cancellation,
        RoutineReuseInput::new(reuse),
    )
    .unwrap()
}

#[test]
pub(crate) fn mediator_fixture_catalog_is_exact_and_claimless() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/routine-public-mediator/cases.json"
    ))
    .unwrap();
    assert_eq!(fixture["schema_version"], "RoutinePublicMediatorCases-v1");
    assert_eq!(fixture["claim_effect"], "none");
    assert_eq!(fixture["production_grant_issuer"], "absent");
    let cases = fixture["cases"].as_array().unwrap();
    let ids = cases
        .iter()
        .map(|case| case["id"].as_str().unwrap())
        .collect::<BTreeSet<_>>();
    assert_eq!(ids.len(), cases.len());
    assert_eq!(ids.len(), 46);
}

#[test]
pub(crate) fn clean_noop_consumes_no_authority_spawns_nothing_and_writes_nothing() {
    let _serial = mediator_lock();
    let fixture = fixture("mediator-clean-noop", false);
    let before_tree = fixture.repo.tree();
    let before_status = fixture.repo.status();
    let before_spawns = test_spawn_count();
    let prepared = prepare_routine_execution(
        &fixture.context,
        &fixture.graph,
        &fixture.snapshot,
        &fixture.plan,
        RoutineAdapterSpec::new("routine", Vec::new()),
    )
    .unwrap();
    let result = mediate(
        &fixture,
        prepared,
        None,
        RoutineCancellation::new(),
        Vec::new(),
    );
    assert_eq!(result.status(), RoutineMediatorStatus::CompleteNoOp);
    assert!(result.request_id().is_none());
    assert!(result.nodes().is_empty());
    assert!(result.reuse_artifacts().is_empty());
    assert!(result.recovery_marker().is_none());
    assert!(result.support_limit().contains("public dispatch"));
    assert_eq!(test_spawn_count(), before_spawns);
    assert_eq!(fixture.repo.tree(), before_tree);
    assert_eq!(fixture.repo.status(), before_status);
}

#[test]
#[cfg(target_os = "macos")]
pub(crate) fn ordered_execution_emits_correlated_results_and_exact_reuse_skips_all_spawns() {
    let _serial = mediator_lock();
    let fixture = fixture("mediator-ordered-reuse", true);
    let original_status = fixture.repo.status();
    let before_spawns = test_spawn_count();
    let first_prepared = prepared(&fixture);
    let first_grant = issue_grant(&first_prepared, "ordered-session-1", None);
    let first = mediate(
        &fixture,
        first_prepared,
        Some(first_grant),
        RoutineCancellation::new(),
        Vec::new(),
    );
    assert_eq!(
        first.status(),
        RoutineMediatorStatus::CompleteExecution,
        "nodes={:?}",
        first.nodes()
    );
    assert_eq!(first.nodes().len(), 3);
    assert_eq!(
        first
            .nodes()
            .iter()
            .map(|node| node.node_id())
            .collect::<Vec<_>>(),
        ["syntax", "compile", "unit"]
    );
    assert!(first.nodes().iter().all(|node| {
        node.disposition() == RoutineNodeDisposition::Executed
            && node.result_artifact_sha256().is_some()
            && node.failure_code().is_none()
    }));
    assert_eq!(first.reuse_artifacts().len(), 3);
    assert_eq!(test_spawn_count(), before_spawns + 3);

    let second_prepared = prepared(&fixture);
    let second_grant = issue_grant(&second_prepared, "ordered-session-2", None);
    let second = mediate(
        &fixture,
        second_prepared,
        Some(second_grant),
        RoutineCancellation::new(),
        first.reuse_artifacts().to_vec(),
    );
    assert_eq!(second.status(), RoutineMediatorStatus::CompleteExecution);
    assert!(
        second
            .nodes()
            .iter()
            .all(|node| node.disposition() == RoutineNodeDisposition::Reused)
    );
    assert_eq!(second.reuse_artifacts(), first.reuse_artifacts());
    assert_eq!(test_spawn_count(), before_spawns + 3);
    assert_eq!(fixture.repo.status(), original_status);
}
