use super::*;

#[test]
pub(crate) fn startup_loader_environment_substitutions_are_rejected_at_preparation() {
    let fixture = dirty_fixture("adapter-startup-loader-environment");
    for key in [
        "ENV",
        "BASH_ENV",
        "RUBYOPT",
        "RUBYLIB",
        "PYTHONPATH",
        "PYTHONSTARTUP",
        "PERL5OPT",
        "PERL5LIB",
        "NODE_OPTIONS",
        "NODE_PATH",
        "CLASSPATH",
        "JAVA_TOOL_OPTIONS",
        "LD_PRELOAD",
        "DYLD_INSERT_LIBRARIES",
        "PHPRC",
        "ZDOTDIR",
    ] {
        let invocation = invocation_specs(&fixture.context, &fixture.plan)
            .remove(0)
            .test_with_environment(BTreeMap::from([
                ("LANG".to_owned(), "C".to_owned()),
                (key.to_owned(), "/tmp/unbound-startup-code".to_owned()),
            ]));
        let mut invocations = invocation_specs(&fixture.context, &fixture.plan);
        invocations[0] = invocation;
        let error = preparation_error(prepare_routine_execution(
            &fixture.context,
            &fixture.graph,
            &fixture.snapshot,
            &fixture.plan,
            RoutineAdapterSpec::new("routine", invocations),
        ));
        assert_eq!(
            error.cause(),
            "adapter-runner-binding-mismatch",
            "key={key}"
        );
    }
}

#[test]
pub(crate) fn missing_runner_identity_and_unbounded_execution_policy_refuse_without_writes() {
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
