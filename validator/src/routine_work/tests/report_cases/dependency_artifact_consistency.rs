use super::*;

#[cfg(target_os = "macos")]
#[test]
pub(crate) fn same_plan_executed_witnesses_cannot_select_different_dependency_artifacts() {
    let _capture = capture_guard();
    let mut repo = TempRepo::new("report-executed-dependency-substitution");
    let (context, plan) = authority_plan(&repo);

    let syntax_expectation = expectation(&context, &plan, "syntax", Vec::new());
    let syntax_a = issue_execution(&repo, &context, &syntax_expectation, b"syntax-a");
    let syntax_a_dependency = syntax_a.work().dependency_result(&context).unwrap();
    let syntax_b = issue_execution(&repo, &context, &syntax_expectation, b"syntax-b");
    let syntax_b_dependency = syntax_b.work().dependency_result(&context).unwrap();

    let compile_a_expectation = expectation(&context, &plan, "compile", vec![syntax_a_dependency]);
    let compile_a = issue_execution(&repo, &context, &compile_a_expectation, b"compile-a");
    let compile_a_dependency = compile_a.work().dependency_result(&context).unwrap();
    let compile_b_expectation = expectation(&context, &plan, "compile", vec![syntax_b_dependency]);
    let compile_b = issue_execution(&repo, &context, &compile_b_expectation, b"compile-b");
    let compile_b_dependency = compile_b.work().dependency_result(&context).unwrap();

    let unit_a_expectation = expectation(&context, &plan, "unit", vec![compile_a_dependency]);
    let unit_a = issue_execution(&repo, &context, &unit_a_expectation, b"unit-a");
    let unit_b_expectation = expectation(&context, &plan, "unit", vec![compile_b_dependency]);
    let unit_b = issue_execution(&repo, &context, &unit_b_expectation, b"unit-b");
    let tree_before_refusal = repo.tree();
    let status_before_refusal = repo.status();

    assert_eq!(
        reconcile_report(
            &context,
            &plan,
            "routine",
            vec![
                ReportRecord::new(
                    "syntax",
                    ReportDisposition::Executed(syntax_a.into_parts().0),
                ),
                ReportRecord::new(
                    "compile",
                    ReportDisposition::Executed(compile_b.into_parts().0),
                ),
                ReportRecord::new("unit", ReportDisposition::Executed(unit_b.into_parts().0),),
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
            vec![
                ReportRecord::new(
                    "syntax",
                    ReportDisposition::Executed(syntax_b.into_parts().0),
                ),
                ReportRecord::new(
                    "compile",
                    ReportDisposition::Executed(compile_a.into_parts().0),
                ),
                ReportRecord::new("unit", ReportDisposition::Executed(unit_a.into_parts().0),),
            ],
        )
        .unwrap_err()
        .id(),
        RoutineErrorId::InvalidRequest
    );
    assert_eq!(repo.tree(), tree_before_refusal);
    assert_eq!(repo.status(), status_before_refusal);
    repo.teardown_after_assertions();
}

#[cfg(target_os = "macos")]
#[test]
pub(crate) fn same_plan_reuse_and_execution_cannot_hide_missing_or_failed_dependencies() {
    let _capture = capture_guard();
    let mut repo = TempRepo::new("report-reuse-dependency-substitution");
    let (context, plan) = authority_plan(&repo);

    let syntax_expectation = expectation(&context, &plan, "syntax", Vec::new());
    let syntax_a = issue_execution(&repo, &context, &syntax_expectation, b"syntax-a");
    let syntax_a_receipt =
        capture_receipt(&repo, &context, syntax_a.receipt_json(), "report-syntax-a");
    let syntax_a_observed = observe_execution(&context, &syntax_expectation);
    let ReuseDecision::Hit(syntax_a_reuse) = assess_reuse(
        &context,
        &syntax_expectation,
        &syntax_a_receipt,
        &syntax_a_observed,
    )
    .unwrap() else {
        panic!("syntax A must verify")
    };
    let syntax_a_dependency = syntax_a.work().dependency_result(&context).unwrap();

    let syntax_b = issue_execution(&repo, &context, &syntax_expectation, b"syntax-b");
    let syntax_b_dependency = syntax_b.work().dependency_result(&context).unwrap();
    let compile_b_expectation = expectation(&context, &plan, "compile", vec![syntax_b_dependency]);
    let compile_b = issue_execution(&repo, &context, &compile_b_expectation, b"compile-b");
    let compile_b_receipt = capture_receipt(
        &repo,
        &context,
        compile_b.receipt_json(),
        "report-compile-b",
    );
    let compile_b_observed = observe_execution(&context, &compile_b_expectation);
    let ReuseDecision::Hit(compile_b_reuse) = assess_reuse(
        &context,
        &compile_b_expectation,
        &compile_b_receipt,
        &compile_b_observed,
    )
    .unwrap() else {
        panic!("compile B must verify")
    };
    let compile_b_dependency = compile_b.work().dependency_result(&context).unwrap();
    let unit_b_expectation = expectation(&context, &plan, "unit", vec![compile_b_dependency]);
    let unit_b = issue_execution(&repo, &context, &unit_b_expectation, b"unit-b");

    assert_eq!(
        reconcile_report(
            &context,
            &plan,
            "routine",
            vec![
                ReportRecord::new("syntax", ReportDisposition::Reused(syntax_a_reuse)),
                ReportRecord::new("compile", ReportDisposition::Reused(compile_b_reuse)),
                ReportRecord::new("unit", ReportDisposition::Executed(unit_b.into_parts().0),),
            ],
        )
        .unwrap_err()
        .id(),
        RoutineErrorId::InvalidRequest
    );

    let compile_a_expectation = expectation(&context, &plan, "compile", vec![syntax_a_dependency]);
    let compile_a = issue_execution(&repo, &context, &compile_a_expectation, b"compile-a");
    assert_eq!(
        reconcile_report(
            &context,
            &plan,
            "routine",
            vec![ReportRecord::new(
                "compile",
                ReportDisposition::Executed(compile_a.into_parts().0),
            )],
        )
        .unwrap_err()
        .id(),
        RoutineErrorId::InvalidRequest
    );

    let failed_syntax_bytes = result_bytes_with(
        &context,
        &syntax_expectation,
        RunOutcome::Failed,
        false,
        sha(b"syntax-failed"),
        BTreeMap::new(),
    );
    let failed_syntax_run = capture_run(&repo, &context, &syntax_expectation, &failed_syntax_bytes);
    let failed_syntax =
        capture_executed_result(&context, &syntax_expectation, &failed_syntax_run).unwrap();
    let compile_from_passed = issue_execution(
        &repo,
        &context,
        &compile_a_expectation,
        b"compile-from-passed-syntax",
    );
    assert_eq!(
        reconcile_report(
            &context,
            &plan,
            "routine",
            vec![
                ReportRecord::new(
                    "syntax",
                    ReportDisposition::Executed(failed_syntax.into_parts().0),
                ),
                ReportRecord::new(
                    "compile",
                    ReportDisposition::Executed(compile_from_passed.into_parts().0),
                ),
            ],
        )
        .unwrap_err()
        .id(),
        RoutineErrorId::InvalidRequest
    );
    repo.teardown_after_assertions();
}
