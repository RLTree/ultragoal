use super::*;

#[cfg(target_os = "macos")]
#[test]
pub(crate) fn complete_execution_requires_mixed_opaque_executed_and_verified_reuse_witnesses() {
    let _capture = capture_guard();
    let repo = TempRepo::new("complete-report");
    let (context, plan) = authority_plan(&repo);

    let syntax_expectation = expectation(&context, &plan, "syntax", Vec::new());
    let syntax_execution = issue_execution(&repo, &context, &syntax_expectation, b"syntax");
    let syntax_receipt = capture_receipt(
        &repo,
        &context,
        syntax_execution.receipt_json(),
        "report-syntax",
    );
    let syntax_observed = observe_execution(&context, &syntax_expectation);
    let ReuseDecision::Hit(syntax_reuse) = assess_reuse(
        &context,
        &syntax_expectation,
        &syntax_receipt,
        &syntax_observed,
    )
    .unwrap() else {
        panic!("exact captured evidence must verify")
    };
    let syntax_dependency = syntax_reuse.dependency_result(&context).unwrap();

    let compile_expectation = expectation(&context, &plan, "compile", vec![syntax_dependency]);
    let compile = issue_execution(&repo, &context, &compile_expectation, b"compile");
    let compile_dependency = compile.work().dependency_result(&context).unwrap();
    let (compile, _) = compile.into_parts();
    let unit_expectation = expectation(&context, &plan, "unit", vec![compile_dependency]);
    let unit = issue_execution(&repo, &context, &unit_expectation, b"unit");
    let (unit, _) = unit.into_parts();

    let report = reconcile_report(
        &context,
        &plan,
        "routine",
        vec![
            ReportRecord::new("syntax", ReportDisposition::Reused(syntax_reuse)),
            ReportRecord::new("compile", ReportDisposition::Executed(compile)),
            ReportRecord::new("unit", ReportDisposition::Executed(unit)),
        ],
    )
    .unwrap();
    assert_eq!(report.status(), ReportStatus::CompleteExecution);
    assert_eq!(
        report.executed(),
        &["compile".to_owned(), "unit".to_owned()]
    );
    assert_eq!(report.reused(), &["syntax".to_owned()]);
    assert_eq!(report.result_scope(), "routine");
    assert_eq!(report.context_id(), context.context_id());
    assert_eq!(report.candidate_id(), plan.binding().candidate_id());
    assert!(report.support_limit().contains("no claim decision"));
}

#[cfg(target_os = "macos")]
#[test]
pub(crate) fn missing_witness_rows_and_partial_failure_remain_incomplete() {
    let _capture = capture_guard();
    let repo = TempRepo::new("partial-report");
    let (context, plan) = authority_plan(&repo);
    let syntax_expectation = expectation(&context, &plan, "syntax", Vec::new());
    let syntax = issue_execution(&repo, &context, &syntax_expectation, b"syntax");
    let (syntax, _) = syntax.into_parts();
    let report = reconcile_report(
        &context,
        &plan,
        "routine",
        vec![
            ReportRecord::new("syntax", ReportDisposition::Executed(syntax)),
            ReportRecord::new(
                "compile",
                ReportDisposition::Skipped(SkipReason::DependencyFailed),
            ),
        ],
    )
    .unwrap();
    assert_eq!(report.status(), ReportStatus::IncompleteExecution);
    assert_eq!(
        report.skipped().get("compile"),
        Some(&SkipReason::DependencyFailed)
    );
    assert_eq!(report.skipped().get("unit"), Some(&SkipReason::Cancelled));
}

#[cfg(target_os = "macos")]
#[test]
pub(crate) fn tracked_mutation_during_report_never_mints_complete_execution() {
    let _capture = capture_guard();
    let repo = TempRepo::new("report-live-mutation");
    let (context, plan) = authority_plan(&repo);
    let syntax_expectation = expectation(&context, &plan, "syntax", Vec::new());
    let execution = issue_execution(&repo, &context, &syntax_expectation, b"syntax");
    let receipt = capture_receipt(&repo, &context, execution.receipt_json(), "report-mutation");
    let observed = observe_execution(&context, &syntax_expectation);
    let ReuseDecision::Hit(first) =
        assess_reuse(&context, &syntax_expectation, &receipt, &observed).unwrap()
    else {
        panic!("exact evidence must verify")
    };
    let syntax_dependency = first.dependency_result(&context).unwrap();
    let compile_expectation = expectation(&context, &plan, "compile", vec![syntax_dependency]);
    let compile = issue_execution(&repo, &context, &compile_expectation, b"compile");
    let compile_dependency = compile.work().dependency_result(&context).unwrap();
    let (compile, _) = compile.into_parts();
    let unit_expectation = expectation(&context, &plan, "unit", vec![compile_dependency]);
    let unit = issue_execution(&repo, &context, &unit_expectation, b"unit");
    let (unit, _) = unit.into_parts();
    let root = repo.root().to_path_buf();
    set_test_live_authority_hook(move || {
        std::fs::write(root.join("src/lib.rs"), b"mutated during report\n").unwrap();
    });
    assert_eq!(
        reconcile_report(
            &context,
            &plan,
            "routine",
            vec![
                ReportRecord::new("syntax", ReportDisposition::Reused(first)),
                ReportRecord::new("compile", ReportDisposition::Executed(compile)),
                ReportRecord::new("unit", ReportDisposition::Executed(unit)),
            ],
        )
        .unwrap_err()
        .id(),
        RoutineErrorId::ConcurrentMutation
    );
    let current = authority_context(&repo);
    assert_eq!(
        reconcile_report(&current, &plan, "routine", Vec::new())
            .unwrap_err()
            .id(),
        RoutineErrorId::ContextMismatch
    );
}

#[cfg(target_os = "macos")]
#[test]
pub(crate) fn unknown_duplicate_wrong_scope_and_invalid_failure_rows_are_rejected() {
    let _capture = capture_guard();
    let repo = TempRepo::new("invalid-report");
    let (context, plan) = authority_plan(&repo);
    let syntax_expectation = expectation(&context, &plan, "syntax", Vec::new());
    let first = issue_execution(&repo, &context, &syntax_expectation, b"syntax-one");
    let second = issue_execution(&repo, &context, &syntax_expectation, b"syntax-two");
    assert_eq!(
        reconcile_report(
            &context,
            &plan,
            "routine",
            vec![
                ReportRecord::new("syntax", ReportDisposition::Executed(first.into_parts().0)),
                ReportRecord::new("syntax", ReportDisposition::Executed(second.into_parts().0)),
            ],
        )
        .unwrap_err()
        .id(),
        RoutineErrorId::InvalidRequest
    );
    assert_eq!(
        reconcile_report(
            &context,
            &plan,
            "routine",
            vec![ReportRecord::new(
                "unknown",
                ReportDisposition::Skipped(SkipReason::Cancelled),
            )],
        )
        .unwrap_err()
        .id(),
        RoutineErrorId::InvalidRequest
    );
    let syntax = issue_execution(&repo, &context, &syntax_expectation, b"syntax");
    assert_eq!(
        reconcile_report(
            &context,
            &plan,
            "wrong-scope",
            vec![ReportRecord::new(
                "syntax",
                ReportDisposition::Executed(syntax.into_parts().0),
            )],
        )
        .unwrap_err()
        .id(),
        RoutineErrorId::InvalidRequest
    );
    assert_eq!(
        reconcile_report(
            &context,
            &plan,
            "routine",
            vec![ReportRecord::new(
                "syntax",
                ReportDisposition::Failed {
                    cause_code: "secret value".to_owned(),
                },
            )],
        )
        .unwrap_err()
        .id(),
        RoutineErrorId::InvalidRequest
    );
}
