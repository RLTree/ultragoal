use std::process::Command;

use super::routine_work::{
    ChangeKind, LocalDirtyTree, PlanMode, PlanRequest, RoutineErrorId, SelectionReason,
};
use super::scenario::{TempRepo, fallback_graph, graph, graph_with_order};

#[test]
fn clean_repository_is_an_exact_no_op() {
    let repo = TempRepo::new("clean-noop");
    let context = repo.context("routine");
    let snapshot = LocalDirtyTree::capture(&context).unwrap();
    let plan =
        super::routine_work::plan_routine(&context, &graph(), &snapshot, PlanRequest::routine())
            .unwrap();
    assert!(snapshot.is_clean());
    assert_eq!(plan.affected_set().mode(), PlanMode::NoOp);
    assert!(plan.checks().is_empty());
}

#[test]
fn ordinary_dirty_subset_is_dependency_closed_without_release_ceremony() {
    let repo = TempRepo::new("dirty-subset");
    repo.write("src/lib.rs", b"pub fn value() -> u8 { 2 }\n");
    let context = repo.context("routine");
    let snapshot = LocalDirtyTree::capture(&context).unwrap();
    let plan =
        super::routine_work::plan_routine(&context, &graph(), &snapshot, PlanRequest::routine())
            .unwrap();
    assert_eq!(plan.affected_set().mode(), PlanMode::Fast);
    assert_eq!(
        plan.affected_set().node_ids(),
        &["syntax".to_owned(), "compile".to_owned(), "unit".to_owned()]
    );
    assert!(
        plan.affected_set()
            .reasons("compile")
            .unwrap()
            .contains(&SelectionReason::DirectImpact)
    );
    assert!(plan.check("release").is_none());
}

#[test]
fn dependency_fanout_runs_downstream_checks_and_prerequisites() {
    let repo = TempRepo::new("fanout");
    repo.write("docs/guide.md", b"changed guide\n");
    let context = repo.context("routine");
    let snapshot = LocalDirtyTree::capture(&context).unwrap();
    let plan =
        super::routine_work::plan_routine(&context, &graph(), &snapshot, PlanRequest::routine())
            .unwrap();
    assert_eq!(
        plan.affected_set().node_ids(),
        &["syntax".to_owned(), "compile".to_owned(), "unit".to_owned()]
    );
    assert!(
        plan.affected_set()
            .reasons("unit")
            .unwrap()
            .contains(&SelectionReason::DownstreamImpact)
    );
}

#[test]
fn untracked_renamed_and_deleted_paths_remain_covered() {
    let untracked = TempRepo::new("untracked");
    untracked.write("tests/new.rs", b"#[test] fn new() {}\n");
    let untracked_context = untracked.context("routine");
    let untracked_snapshot = LocalDirtyTree::capture(&untracked_context).unwrap();
    assert!(
        untracked_snapshot
            .changes()
            .iter()
            .any(|change| change.kind() == ChangeKind::Untracked)
    );
    let untracked_plan = super::routine_work::plan_routine(
        &untracked_context,
        &graph(),
        &untracked_snapshot,
        PlanRequest::routine(),
    )
    .unwrap();
    assert!(untracked_plan.check("unit").is_some());

    let renamed = TempRepo::new("renamed");
    renamed.git(&["mv", "docs/guide.md", "src/guide.md"]);
    let renamed_context = renamed.context("routine");
    let renamed_snapshot = LocalDirtyTree::capture(&renamed_context).unwrap();
    let rename = renamed_snapshot
        .changes()
        .iter()
        .find(|change| change.kind() == ChangeKind::Renamed)
        .unwrap();
    assert_eq!(rename.previous_path().unwrap().as_str(), "docs/guide.md");
    let renamed_plan = super::routine_work::plan_routine(
        &renamed_context,
        &graph(),
        &renamed_snapshot,
        PlanRequest::routine(),
    )
    .unwrap();
    assert!(renamed_plan.check("compile").is_some());

    let deleted = TempRepo::new("deleted");
    deleted.remove("src/lib.rs");
    let deleted_context = deleted.context("routine");
    let deleted_snapshot = LocalDirtyTree::capture(&deleted_context).unwrap();
    assert!(
        deleted_snapshot
            .changes()
            .iter()
            .any(|change| change.kind() == ChangeKind::Deleted)
    );
    let deleted_plan = super::routine_work::plan_routine(
        &deleted_context,
        &graph(),
        &deleted_snapshot,
        PlanRequest::routine(),
    )
    .unwrap();
    assert!(deleted_plan.check("compile").is_some());
}

#[test]
fn unknown_paths_and_conflicts_expand_to_strict_all_node_boundary() {
    let repo = TempRepo::new("unknown");
    repo.write("mystery.bin", b"unknown\n");
    let context = repo.context("routine");
    let snapshot = LocalDirtyTree::capture(&context).unwrap();
    let plan =
        super::routine_work::plan_routine(&context, &graph(), &snapshot, PlanRequest::routine())
            .unwrap();
    assert_eq!(plan.affected_set().mode(), PlanMode::Strict);
    assert_eq!(plan.checks().len(), 4);
    assert_eq!(plan.affected_set().coverage().unknown_path_count(), 1);

    let conflict_repo = TempRepo::new("conflict-expansion");
    conflict_repo.git(&["checkout", "-q", "-b", "conflict-side"]);
    conflict_repo.write("src/lib.rs", b"side change\n");
    conflict_repo.git(&["add", "src/lib.rs"]);
    conflict_repo.git(&["commit", "-q", "-m", "side change"]);
    conflict_repo.git(&["checkout", "-q", "-"]);
    conflict_repo.write("src/lib.rs", b"main change\n");
    conflict_repo.git(&["add", "src/lib.rs"]);
    conflict_repo.git(&["commit", "-q", "-m", "main change"]);
    let merge = Command::new("git")
        .args(["merge", "--no-edit", "conflict-side"])
        .current_dir(conflict_repo.root())
        .env("GIT_OPTIONAL_LOCKS", "0")
        .output()
        .unwrap();
    assert!(!merge.status.success(), "fixture must create a conflict");
    let conflict_context = conflict_repo.context("routine");
    let snapshot = LocalDirtyTree::capture(&conflict_context).unwrap();
    assert!(
        snapshot
            .changes()
            .iter()
            .any(|change| change.kind() == ChangeKind::Conflict)
    );
    let conflict_plan = super::routine_work::plan_routine(
        &conflict_context,
        &graph(),
        &snapshot,
        PlanRequest::routine(),
    )
    .unwrap();
    assert_eq!(conflict_plan.affected_set().mode(), PlanMode::Strict);
    assert_eq!(conflict_plan.checks().len(), 4);
}

#[test]
fn strict_named_boundary_and_optional_fallback_are_explicit() {
    let repo = TempRepo::new("strict");
    let context = repo.context("routine");
    let clean = LocalDirtyTree::capture(&context).unwrap();
    let strict = super::routine_work::plan_routine(
        &context,
        &graph(),
        &clean,
        PlanRequest::strict_claim("release"),
    )
    .unwrap();
    assert_eq!(strict.affected_set().mode(), PlanMode::Strict);
    assert_eq!(strict.checks().len(), 4);

    repo.write("src/lib.rs", b"pub fn value() -> u8 { 3 }\n");
    let changed_context = repo.context("routine");
    let changed = LocalDirtyTree::capture(&changed_context).unwrap();
    let fallback = super::routine_work::plan_routine(
        &changed_context,
        &fallback_graph(),
        &changed,
        PlanRequest::routine(),
    )
    .unwrap();
    assert!(fallback.check("compile").unwrap().used_fallback());
    assert_eq!(fallback.check("compile").unwrap().selected_tool(), "git");
    assert_eq!(fallback.affected_set().coverage().fallback_tool_count(), 1);
}

#[test]
fn registry_and_plan_order_are_deterministic_and_unknown_requests_fail() {
    let forward = graph_with_order(false);
    let reverse = graph_with_order(true);
    assert_eq!(forward.graph_id(), reverse.graph_id());
    let repo = TempRepo::new("deterministic");
    repo.write("src/lib.rs", b"pub fn value() -> u8 { 4 }\n");
    let context = repo.context("routine");
    let snapshot = LocalDirtyTree::capture(&context).unwrap();
    let first =
        super::routine_work::plan_routine(&context, &forward, &snapshot, PlanRequest::routine())
            .unwrap();
    let second =
        super::routine_work::plan_routine(&context, &reverse, &snapshot, PlanRequest::routine())
            .unwrap();
    assert_eq!(first.plan_id(), second.plan_id());
    let error = super::routine_work::plan_routine(
        &context,
        &forward,
        &snapshot,
        PlanRequest::routine_with_nodes(vec!["unknown".to_owned()]),
    )
    .unwrap_err();
    assert_eq!(error.id(), RoutineErrorId::UnknownRegistryRow);
}
