use super::routine_work::{
    CheckClass, CheckNode, ImpactGraph, PathMatcher, PathRoute, PlanRequest, RepoPath,
    RoutineErrorId, RunnerSpec, set_test_live_authority_hook,
};
use super::scenario::{TempRepo, graph, node, path, route};

#[test]
fn repository_paths_reject_escape_alias_and_excessive_depth_without_echo() {
    for value in [
        "../secret-token",
        "/absolute",
        "src//file",
        "src\\file",
        ".",
    ] {
        let error = RepoPath::parse(value).unwrap_err();
        assert_eq!(error.id(), RoutineErrorId::InvalidPath);
        assert!(!format!("{error:?}").contains(value));
    }
    let deep = std::iter::repeat_n("d", 65).collect::<Vec<_>>().join("/");
    let error = RepoPath::parse(&deep).unwrap_err();
    assert_eq!(error.id(), RoutineErrorId::InvalidPath);
    assert!(!format!("{error:?}").contains(&deep));
}

#[test]
fn duplicate_unknown_cycle_and_case_alias_registry_rows_fail_closed() {
    let duplicate = ImpactGraph::new(
        vec![
            node("same", &[], CheckClass::Routine, "git", None),
            node("same", &[], CheckClass::Routine, "git", None),
        ],
        Vec::new(),
        Vec::new(),
    )
    .unwrap_err();
    assert_eq!(duplicate.id(), RoutineErrorId::InvalidRegistry);

    let unknown = ImpactGraph::new(
        vec![node("known", &[], CheckClass::Routine, "git", None)],
        vec![route(
            "unknown-target",
            PathMatcher::Exact(path("src/lib.rs")),
            &["missing"],
            false,
        )],
        Vec::new(),
    )
    .unwrap_err();
    assert_eq!(unknown.id(), RoutineErrorId::UnknownRegistryRow);

    let cycle = ImpactGraph::new(
        vec![
            node("a", &["b"], CheckClass::Routine, "git", None),
            node("b", &["a"], CheckClass::Routine, "git", None),
        ],
        Vec::new(),
        Vec::new(),
    )
    .unwrap_err();
    assert_eq!(cycle.id(), RoutineErrorId::InvalidRegistry);

    let case_alias = ImpactGraph::new(
        vec![node("check", &[], CheckClass::Routine, "git", None)],
        vec![
            route("lower", PathMatcher::Prefix(path("src")), &["check"], false),
            route("upper", PathMatcher::Prefix(path("SRC")), &["check"], false),
        ],
        Vec::new(),
    )
    .unwrap_err();
    assert_eq!(case_alias.id(), RoutineErrorId::AmbiguousRegistry);
}

#[test]
fn graph_and_dependency_work_are_bounded() {
    let nodes = (0..4_097)
        .map(|index| {
            CheckNode::new(
                format!("check-{index}"),
                Vec::<String>::new(),
                CheckClass::Routine,
                RunnerSpec::new("git", None).unwrap(),
            )
            .unwrap()
        })
        .collect();
    let error = ImpactGraph::new(nodes, Vec::new(), Vec::new()).unwrap_err();
    assert_eq!(error.id(), RoutineErrorId::InvalidRegistry);

    let dependencies = (0..1_025).map(|index| format!("dep-{index}"));
    let error = CheckNode::new(
        "too-many-dependencies",
        dependencies,
        CheckClass::Routine,
        RunnerSpec::new("git", None).unwrap(),
    )
    .unwrap_err();
    assert_eq!(error.id(), RoutineErrorId::InvalidRegistry);
}

#[test]
fn duplicated_dependency_route_and_explicit_request_rows_are_rejected() {
    let duplicate_dependency = CheckNode::new(
        "check",
        vec!["dep".to_owned(), "dep".to_owned()],
        CheckClass::Routine,
        RunnerSpec::new("git", None).unwrap(),
    )
    .unwrap_err();
    assert_eq!(duplicate_dependency.id(), RoutineErrorId::InvalidRegistry);
    let duplicate_route_target = PathRoute::new(
        "route",
        PathMatcher::Exact(path("src/lib.rs")),
        vec!["check".to_owned(), "check".to_owned()],
        false,
    )
    .unwrap_err();
    assert_eq!(duplicate_route_target.id(), RoutineErrorId::InvalidRegistry);

    let mut repo = TempRepo::new("duplicated-explicit-request");
    repo.write("src/lib.rs", b"changed\n");
    let context = repo.context("routine");
    let snapshot = super::routine_work::LocalDirtyTree::capture(&context).unwrap();
    let error = super::routine_work::plan_routine(
        &context,
        &graph(),
        &snapshot,
        PlanRequest::routine_with_nodes(vec!["compile".to_owned(), "compile".to_owned()]),
    )
    .unwrap_err();
    assert_eq!(error.id(), RoutineErrorId::InvalidRequest);
    repo.teardown_after_assertions();
}

#[test]
fn candidate_change_during_planning_invalidates_the_plan() {
    let mut repo = TempRepo::new("planning-live-mutation");
    repo.write("src/lib.rs", b"first candidate\n");
    let context = repo.context("routine");
    let snapshot = super::routine_work::LocalDirtyTree::capture(&context).unwrap();
    let root = repo.root().to_path_buf();
    set_test_live_authority_hook(move || {
        std::fs::write(root.join("src/lib.rs"), b"second candidate\n").unwrap();
    });
    let error =
        super::routine_work::plan_routine(&context, &graph(), &snapshot, PlanRequest::routine())
            .unwrap_err();
    assert_eq!(error.id(), RoutineErrorId::ConcurrentMutation);
    repo.teardown_after_assertions();
}

#[test]
fn semantic_identifier_errors_hash_sensitive_subjects() {
    let secret = "private identifier with spaces";
    let error = RunnerSpec::new(secret, None).unwrap_err();
    assert_eq!(error.id(), RoutineErrorId::InvalidRegistry);
    assert!(!format!("{error}").contains(secret));
    assert!(error.subject_sha256().is_some());
}
