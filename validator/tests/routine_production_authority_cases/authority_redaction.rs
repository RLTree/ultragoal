use super::*;

#[test]
pub(crate) fn secrets_paths_and_raw_output_are_absent_from_authority_and_diagnostics() {
    let fixture = fixture("production-redaction", true);
    let authority = AuthorityRoot::new("production-redaction");
    let issuer = ProductionRoutineIssuer::open(authority.path()).unwrap();
    let request = prepared(&fixture);
    issuer
        .test_reserve_and_abandon(&fixture.context, &fixture.plan, &request, false)
        .unwrap();
    let key = fs::read(authority.path().join("routine-authority.key")).unwrap();
    let state = fs::read(authority.path().join("routine-authority.state")).unwrap();
    let state_text = String::from_utf8(state.clone()).unwrap();
    assert!(!state.windows(key.len()).any(|window| window == key));
    assert!(!state_text.contains(&fixture.repo.root().to_string_lossy().as_ref()));
    assert!(!state_text.contains("compile\n"));
    let error = issuer
        .mediate(
            &fixture.context,
            &fixture.plan,
            request,
            None,
            RoutineCancellation::new(),
            RoutineReuseInput::new(Vec::new()),
        )
        .unwrap_err();
    let diagnostic = error.to_string();
    assert!(!diagnostic.contains(&authority.path().to_string_lossy().as_ref()));
    assert!(!diagnostic.contains(&fixture.repo.root().to_string_lossy().as_ref()));
}

#[test]
pub(crate) fn production_boundary_has_one_sealed_issuer_and_no_test_grant_entrypoint() {
    let runtime = include_str!("../src/routine_work/runtime_adapter/mod.rs");
    let production = include_str!("../src/routine_work/runtime_adapter/production/mod.rs");
    let ledger = include_str!("../src/routine_work/runtime_adapter/production/ledger/mod.rs");
    let model = include_str!("../src/routine_work/runtime_adapter/mediator/model.rs");
    assert_eq!(production.matches("issue_production_grant(").count(), 1);
    assert!(production.contains("pub(crate) struct ProductionRoutineIssuer"));
    assert!(production.contains("preflight_production_request"));
    assert!(!production.contains("RoutineRootGrant::test_issue"));
    assert!(model.contains("#[cfg(test)]\nimpl RoutineRootGrant"));
    assert!(runtime.contains("mod production;"));
    assert!(!production.contains("ClaimDecision"));
    assert!(!production.contains("public command"));
    assert!(ledger.contains("pub(crate) struct ReusePreauthorization"));
    let marker = "pub(crate) struct ReusePreauthorization";
    let offset = ledger.find(marker).unwrap();
    let attributes = &ledger[offset.saturating_sub(180)..offset];
    let authorization = ledger[offset + marker.len()..]
        .split("\n}\n")
        .next()
        .unwrap();
    assert!(!attributes.contains("derive(Clone"));
    assert!(!attributes.contains("Serialize"));
    assert!(!attributes.contains("Deserialize"));
    assert!(!authorization.contains("pub("));
    assert_eq!(ledger.matches("ReusePreauthorization {").count(), 2);
}

#[test]
#[ignore = "spawned explicitly by the cross-process race test"]
pub(crate) fn production_child_race_attempt() {
    if std::env::var_os("HUL_ROUTINE_PRODUCTION_CHILD").is_none() {
        return;
    }
    let repo_root = PathBuf::from(std::env::var_os("HUL_ROUTINE_REPO").unwrap());
    let authority_root = PathBuf::from(std::env::var_os("HUL_ROUTINE_AUTHORITY").unwrap());
    let outcome = PathBuf::from(std::env::var_os("HUL_ROUTINE_OUTCOME").unwrap());
    let context = LiveContext::build(
        BuildRequest::new(&repo_root)
            .bind_non_secret_configuration("profile", "routine-production-race")
            .probe_tool("sandbox-exec")
            .probe_tool("dash"),
    )
    .unwrap();
    let snapshot = LocalDirtyTree::capture(&context).unwrap();
    let graph = graph();
    let plan = plan_routine(&context, &graph, &snapshot, PlanRequest::routine()).unwrap();
    let invocation = bind_routine_invocation(
        &context,
        &plan,
        "compile",
        vec!["-c".to_owned(), command_script("compile")],
        10_000,
        1024 * 1024,
        vec![path("target/routine/compile")],
    )
    .unwrap();
    let prepared = prepare_routine_execution(
        &context,
        &graph,
        &snapshot,
        &plan,
        RoutineAdapterSpec::new("routine", vec![invocation]),
    )
    .unwrap();
    let reuse = std::env::var_os("HUL_ROUTINE_REUSE")
        .map(|path| vec![fs::read(path).unwrap()])
        .unwrap_or_default();
    let result = mediate_prepared_routine_execution_production(
        &authority_root,
        &context,
        &plan,
        prepared,
        None,
        RoutineCancellation::new(),
        RoutineReuseInput::new(reuse),
    );
    let text = match result {
        Ok(_) => "complete".to_owned(),
        Err(error) => error.cause().to_owned(),
    };
    fs::write(outcome, text).unwrap();
}

#[test]
pub(crate) fn two_processes_racing_the_same_protocol_have_exactly_one_winner() {
    let repo = TempRepo::new("production-process-race");
    repo.write(".git/info/exclude", b"target/\n");
    fs::create_dir_all(repo.root().join("target/routine/compile")).unwrap();
    repo.write("src/lib.rs", b"pub fn value() -> u8 { 97 }\n");
    let authority = AuthorityRoot::new("production-process-race");
    let executable = std::env::current_exe().unwrap();
    let outcome_a = authority.parent.join("outcome-a");
    let outcome_b = authority.parent.join("outcome-b");
    let spawn = |outcome: &Path| {
        Command::new(&executable)
            .args([
                "--ignored",
                "--exact",
                "production_child_race_attempt",
                "--nocapture",
            ])
            .env("HUL_ROUTINE_PRODUCTION_CHILD", "1")
            .env("HUL_ROUTINE_REPO", repo.root())
            .env("HUL_ROUTINE_AUTHORITY", authority.path())
            .env("HUL_ROUTINE_OUTCOME", outcome)
            .spawn()
            .unwrap()
    };
    let mut child_a = spawn(&outcome_a);
    let mut child_b = spawn(&outcome_b);
    assert!(child_a.wait().unwrap().success());
    assert!(child_b.wait().unwrap().success());
    let outcomes = [
        fs::read_to_string(outcome_a).unwrap(),
        fs::read_to_string(outcome_b).unwrap(),
    ];
    assert_eq!(
        outcomes.iter().filter(|value| *value == "complete").count(),
        1
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|value| value.contains("replayed"))
            .count(),
        1,
        "outcomes={outcomes:?}"
    );
}
