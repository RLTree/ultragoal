use super::*;

#[test]
pub(crate) fn clean_noop_is_deterministic_exact_and_non_effectful() {
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
pub(crate) fn dirty_preparation_binds_deterministic_complete_typed_intents_without_effects() {
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
        assert_eq!(intent.argv(), ["ultragoal", "--json", "check", "routine"]);
        assert_eq!(
            intent.working_directory(),
            fixture.context.worktree_root().to_str().unwrap()
        );
        assert_eq!(intent.environment_policy(), "clear-all-allowlisted-v1");
        assert_eq!(intent.environment_keys(), ["LANG", "LC_ALL", "PATH"]);
        assert!(intent.environment_sha256().starts_with("sha256:"));
        assert_eq!(
            intent.read_authority_policy(),
            "default-deny-exact-bound-read-v1"
        );
        assert_eq!(intent.read_source_paths(), [path("src/lib.rs")]);
        assert!(intent.read_authority_sha256().starts_with("sha256:"));
        assert_eq!(
            intent.mediation_preflight(),
            "revalidate-context-candidate-tool-executable-read-sources-output-scopes-before-and-after-effect-v1"
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
}
