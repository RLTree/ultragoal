use super::*;

#[test]
pub(crate) fn malformed_invocation_sets_and_stale_bindings_refuse_without_writes() {
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

    let mut read_authority_substitution = invocation_specs(&fixture.context, &fixture.plan);
    let first = read_authority_substitution
        .remove(0)
        .test_with_read_authority_sha256(format!("sha256:{}", "2".repeat(64)));
    read_authority_substitution.insert(0, first);
    assert_eq!(
        preparation_error(prepare_routine_execution(
            &fixture.context,
            &fixture.graph,
            &fixture.snapshot,
            &fixture.plan,
            RoutineAdapterSpec::new("routine", read_authority_substitution),
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

    let mut environment_substitution = invocation_specs(&fixture.context, &fixture.plan);
    let first = environment_substitution
        .remove(0)
        .test_with_environment(BTreeMap::from([
            ("LANG".to_owned(), "C".to_owned()),
            ("LC_ALL".to_owned(), "C".to_owned()),
            ("PATH".to_owned(), "/unbound".to_owned()),
        ]));
    environment_substitution.insert(0, first);
    assert_eq!(
        preparation_error(prepare_routine_execution(
            &fixture.context,
            &fixture.graph,
            &fixture.snapshot,
            &fixture.plan,
            RoutineAdapterSpec::new("routine", environment_substitution),
        ))
        .cause(),
        "adapter-runner-binding-mismatch"
    );

    assert_eq!(
        bind_routine_invocation_with_environment(
            &fixture.context,
            &fixture.plan,
            "syntax",
            vec!["--reserved-environment".to_owned()],
            BTreeMap::from([("HUL_ROUTINE_REQUEST_ID".to_owned(), "forged".to_owned())]),
            1_000,
            1_024,
            vec![path("target/routine")],
        )
        .unwrap_err()
        .cause(),
        "adapter-environment-invalid"
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
