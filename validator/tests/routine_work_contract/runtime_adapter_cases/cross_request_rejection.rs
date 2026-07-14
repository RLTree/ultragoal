use super::*;

#[cfg(target_os = "macos")]
#[test]
pub(crate) fn cross_request_witnesses_and_inconsistent_dependency_artifacts_fail_closed() {
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
pub(crate) fn candidate_mutation_during_delegated_reconciliation_never_returns_complete() {
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
pub(crate) fn preparation_and_pre_report_refusal_source_has_no_effect_primitive() {
    let adapter = include_str!("../../src/routine_work/runtime_adapter/mod.rs");
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
