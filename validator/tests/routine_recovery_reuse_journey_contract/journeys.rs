use std::process::Command;

use super::base_support::{TempRepo, fallback_graph, graph};
use super::journey_support::{
    authority_context, authority_plan, capture_guard, capture_receipt, expectation,
    issue_execution, observe_execution,
};
use super::routine_work::{
    ChangeKind, LocalDirtyTree, PlanMode, PlanRequest, ReportDisposition, ReportRecord,
    ReportStatus, ReuseDecision, ReuseMiss, RoutineErrorId, assess_reuse, reconcile_report,
};

#[test]
fn clean_dirty_conflict_and_fallback_journeys_preserve_unrelated_work() {
    let clean = TempRepo::new("journey-clean");
    let clean_context = clean.context("routine");
    let clean_tree = clean.tree();
    let clean_status = clean.status();
    let clean_snapshot = LocalDirtyTree::capture(&clean_context).unwrap();
    let clean_plan = super::routine_work::plan_routine(
        &clean_context,
        &graph(),
        &clean_snapshot,
        PlanRequest::routine(),
    )
    .unwrap();
    assert_eq!(clean_plan.affected_set().mode(), PlanMode::NoOp);
    assert_eq!(clean.tree(), clean_tree);
    assert_eq!(clean.status(), clean_status);

    let dirty = TempRepo::new("journey-dirty");
    dirty.write("src/lib.rs", b"dirty source\n");
    dirty.write("docs/guide.md", b"operator work\n");
    let dirty_context = dirty.context("routine");
    let dirty_status = dirty.status();
    let unrelated = std::fs::read(dirty.root().join("docs/guide.md")).unwrap();
    let dirty_snapshot = LocalDirtyTree::capture(&dirty_context).unwrap();
    let dirty_plan = super::routine_work::plan_routine(
        &dirty_context,
        &graph(),
        &dirty_snapshot,
        PlanRequest::routine(),
    )
    .unwrap();
    assert_eq!(dirty_plan.affected_set().mode(), PlanMode::Fast);
    assert_eq!(dirty_plan.checks().len(), 3);
    assert_eq!(dirty.status(), dirty_status);
    assert_eq!(
        std::fs::read(dirty.root().join("docs/guide.md")).unwrap(),
        unrelated
    );

    let conflict = TempRepo::new("journey-conflict");
    conflict.git(&["checkout", "-q", "-b", "journey-side"]);
    conflict.write("src/lib.rs", b"side\n");
    conflict.git(&["add", "src/lib.rs"]);
    conflict.git(&["commit", "-q", "-m", "side"]);
    conflict.git(&["checkout", "-q", "-"]);
    conflict.write("src/lib.rs", b"main\n");
    conflict.git(&["add", "src/lib.rs"]);
    conflict.git(&["commit", "-q", "-m", "main"]);
    let merge = Command::new("git")
        .args(["merge", "--no-edit", "journey-side"])
        .current_dir(conflict.root())
        .env("GIT_OPTIONAL_LOCKS", "0")
        .output()
        .unwrap();
    assert!(!merge.status.success());
    let conflict_context = conflict.context("routine");
    let conflict_status = conflict.status();
    let conflict_snapshot = LocalDirtyTree::capture(&conflict_context).unwrap();
    assert!(
        conflict_snapshot
            .changes()
            .iter()
            .any(|change| change.kind() == ChangeKind::Conflict)
    );
    let conflict_plan = super::routine_work::plan_routine(
        &conflict_context,
        &graph(),
        &conflict_snapshot,
        PlanRequest::routine(),
    )
    .unwrap();
    assert_eq!(conflict_plan.affected_set().mode(), PlanMode::Strict);
    assert_eq!(conflict_plan.checks().len(), 4);
    assert_eq!(conflict.status(), conflict_status);

    let fallback = TempRepo::new("journey-fallback");
    fallback.write("src/lib.rs", b"fallback\n");
    let fallback_context = fallback.context("routine");
    let fallback_status = fallback.status();
    let fallback_snapshot = LocalDirtyTree::capture(&fallback_context).unwrap();
    let fallback_plan = super::routine_work::plan_routine(
        &fallback_context,
        &fallback_graph(),
        &fallback_snapshot,
        PlanRequest::routine(),
    )
    .unwrap();
    let selected = fallback_plan.check("compile").unwrap();
    assert!(selected.used_fallback());
    assert_eq!(selected.selected_tool(), "git");
    assert_eq!(
        fallback_plan
            .affected_set()
            .coverage()
            .fallback_tool_count(),
        1
    );
    assert_eq!(fallback.status(), fallback_status);
}

#[cfg(target_os = "macos")]
#[test]
fn first_execution_exact_repeat_reuse_and_mutation_forced_miss_are_causal() {
    let _capture = capture_guard();
    let repo = TempRepo::new("journey-repeat-reuse");
    let (context, plan) = authority_plan(&repo);
    let baseline_status = repo.status();
    let unrelated_path = repo.root().join("docs/guide.md");
    let unrelated_bytes = std::fs::read(&unrelated_path).unwrap();

    let syntax_expectation = expectation(&context, &plan, "syntax", Vec::new());
    let syntax = issue_execution(&repo, &context, &syntax_expectation, b"syntax");
    let syntax_receipt = capture_receipt(&repo, &context, syntax.receipt_json(), "journey-syntax");
    let syntax_observed = observe_execution(&context, &syntax_expectation);
    let syntax_dependency = syntax.work().dependency_result(&context).unwrap();

    let compile_expectation = expectation(&context, &plan, "compile", vec![syntax_dependency]);
    let compile = issue_execution(&repo, &context, &compile_expectation, b"compile");
    let compile_receipt =
        capture_receipt(&repo, &context, compile.receipt_json(), "journey-compile");
    let compile_observed = observe_execution(&context, &compile_expectation);
    let compile_dependency = compile.work().dependency_result(&context).unwrap();

    let unit_expectation = expectation(&context, &plan, "unit", vec![compile_dependency]);
    let unit = issue_execution(&repo, &context, &unit_expectation, b"unit");
    let unit_receipt = capture_receipt(&repo, &context, unit.receipt_json(), "journey-unit");
    let unit_observed = observe_execution(&context, &unit_expectation);

    let first = reconcile_report(
        &context,
        &plan,
        "routine",
        vec![
            ReportRecord::new("syntax", ReportDisposition::Executed(syntax.into_parts().0)),
            ReportRecord::new(
                "compile",
                ReportDisposition::Executed(compile.into_parts().0),
            ),
            ReportRecord::new("unit", ReportDisposition::Executed(unit.into_parts().0)),
        ],
    )
    .unwrap();
    assert_eq!(first.status(), ReportStatus::CompleteExecution);
    assert_eq!(first.executed().len(), 3);

    let tree_before_repeat = repo.tree();
    let status_before_repeat = repo.status();
    let ReuseDecision::Hit(syntax_reuse) = assess_reuse(
        &context,
        &syntax_expectation,
        &syntax_receipt,
        &syntax_observed,
    )
    .unwrap() else {
        panic!("syntax exact repeat must reuse")
    };
    let ReuseDecision::Hit(compile_reuse) = assess_reuse(
        &context,
        &compile_expectation,
        &compile_receipt,
        &compile_observed,
    )
    .unwrap() else {
        panic!("compile exact repeat must reuse")
    };
    let ReuseDecision::Hit(unit_reuse) =
        assess_reuse(&context, &unit_expectation, &unit_receipt, &unit_observed).unwrap()
    else {
        panic!("unit exact repeat must reuse")
    };
    let repeat = reconcile_report(
        &context,
        &plan,
        "routine",
        vec![
            ReportRecord::new("unit", ReportDisposition::Reused(unit_reuse)),
            ReportRecord::new("syntax", ReportDisposition::Reused(syntax_reuse)),
            ReportRecord::new("compile", ReportDisposition::Reused(compile_reuse)),
        ],
    )
    .unwrap();
    assert_eq!(repeat.status(), ReportStatus::CompleteExecution);
    assert_eq!(repeat.reused().len(), 3);
    assert!(repeat.executed().is_empty());
    assert_eq!(repo.tree(), tree_before_repeat);
    assert_eq!(repo.status(), status_before_repeat);
    assert_eq!(repo.status(), baseline_status);
    assert_eq!(std::fs::read(&unrelated_path).unwrap(), unrelated_bytes);

    repo.write("src/lib.rs", b"mutation forces miss\n");
    assert_eq!(
        assess_reuse(
            &context,
            &syntax_expectation,
            &syntax_receipt,
            &syntax_observed,
        )
        .unwrap_err()
        .id(),
        RoutineErrorId::ConcurrentMutation
    );
    let fresh = authority_context(&repo);
    assert_eq!(
        assess_reuse(
            &fresh,
            &syntax_expectation,
            &syntax_receipt,
            &syntax_observed,
        )
        .unwrap(),
        ReuseDecision::Miss(ReuseMiss::Candidate)
    );
    assert_eq!(std::fs::read(&unrelated_path).unwrap(), unrelated_bytes);
}
