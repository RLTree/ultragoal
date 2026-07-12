use super::reuse::common::authority_context;
use super::reuse::common::{
    authority_plan, capture_guard, capture_receipt, capture_run, expectation, issue_execution,
    observe_execution, result_bytes_with,
};
use super::routine_work::{
    ReportDisposition, ReportRecord, ReportStatus, ReuseDecision, RoutineErrorId, RunOutcome,
    SkipReason, assess_reuse, capture_executed_result, reconcile_report,
    set_test_live_authority_hook,
};
use super::support::{TempRepo, sha};
use std::collections::BTreeMap;

#[cfg(target_os = "macos")]
#[test]
fn complete_execution_requires_mixed_opaque_executed_and_verified_reuse_witnesses() {
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
fn missing_witness_rows_and_partial_failure_remain_incomplete() {
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
fn tracked_mutation_during_report_never_mints_complete_execution() {
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
fn unknown_duplicate_wrong_scope_and_invalid_failure_rows_are_rejected() {
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

#[cfg(target_os = "macos")]
#[test]
fn same_plan_executed_witnesses_cannot_select_different_dependency_artifacts() {
    let _capture = capture_guard();
    let repo = TempRepo::new("report-executed-dependency-substitution");
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
}

#[cfg(target_os = "macos")]
#[test]
fn same_plan_reuse_and_execution_cannot_hide_missing_or_failed_dependencies() {
    let _capture = capture_guard();
    let repo = TempRepo::new("report-reuse-dependency-substitution");
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
}
