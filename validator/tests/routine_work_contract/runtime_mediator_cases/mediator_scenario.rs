use super::*;

pub(crate) struct MediatorFixture {
    pub(crate) repo: TempRepo,
    pub(crate) context: LiveContext,
    pub(crate) graph: ImpactGraph,
    pub(crate) snapshot: DirtySnapshot,
    pub(crate) plan: RoutinePlan,
}

pub(crate) fn mediator_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

pub(crate) fn mediator_context(repo: &TempRepo, profile: &str) -> LiveContext {
    mediator_context_for_tool(repo, profile, "dash")
}

pub(crate) fn mediator_context_for_tool(repo: &TempRepo, profile: &str, tool: &str) -> LiveContext {
    LiveContext::build(
        BuildRequest::new(repo.root())
            .bind_non_secret_configuration("profile", profile)
            .probe_tool("sandbox-exec")
            .probe_tool(tool),
    )
    .unwrap()
}

pub(crate) fn mediator_graph() -> ImpactGraph {
    mediator_graph_for_tool("dash")
}

pub(crate) fn mediator_graph_for_tool(tool: &str) -> ImpactGraph {
    ImpactGraph::new(
        vec![
            node("syntax", &[], CheckClass::Routine, tool, None),
            node("compile", &["syntax"], CheckClass::Routine, tool, None),
            node("unit", &["compile"], CheckClass::Routine, tool, None),
        ],
        vec![
            route(
                "route-src",
                PathMatcher::Prefix(path("src")),
                &["compile"],
                false,
            ),
            route(
                "route-release",
                PathMatcher::Exact(path("release.json")),
                &["unit"],
                true,
            ),
        ],
        Vec::new(),
    )
    .unwrap()
}

pub(crate) fn strict_fixture(label: &str) -> MediatorFixture {
    let repo = TempRepo::new(label);
    repo.write(".git/info/exclude", b"target/\n");
    for node_id in ["syntax", "compile", "unit"] {
        fs::create_dir_all(repo.root().join(format!("target/routine/{node_id}"))).unwrap();
    }
    repo.write("release.json", b"{\"strict\":true}\n");
    let context = mediator_context(&repo, "routine-mediator-strict");
    let snapshot = LocalDirtyTree::capture(&context).unwrap();
    let graph = mediator_graph();
    let plan = plan_routine(&context, &graph, &snapshot, PlanRequest::routine()).unwrap();
    assert_eq!(plan.affected_set().mode(), PlanMode::Strict);
    MediatorFixture {
        repo,
        context,
        graph,
        snapshot,
        plan,
    }
}

pub(crate) fn fallback_fixture(label: &str) -> MediatorFixture {
    let repo = TempRepo::new(label);
    repo.write(".git/info/exclude", b"target/\n");
    fs::create_dir_all(repo.root().join("target/routine/compile")).unwrap();
    repo.write("src/lib.rs", b"pub fn value() -> u8 { 61 }\n");
    let context = LiveContext::build(
        BuildRequest::new(repo.root())
            .bind_non_secret_configuration("profile", "routine-mediator-fallback")
            .probe_tool("sandbox-exec")
            .probe_tool("definitely-missing-routine-mediator")
            .probe_tool("dash"),
    )
    .unwrap();
    let snapshot = LocalDirtyTree::capture(&context).unwrap();
    let graph = ImpactGraph::new(
        vec![node(
            "compile",
            &[],
            CheckClass::Routine,
            "definitely-missing-routine-mediator",
            Some("dash"),
        )],
        vec![route(
            "route-src",
            PathMatcher::Prefix(path("src")),
            &["compile"],
            false,
        )],
        Vec::new(),
    )
    .unwrap();
    let plan = plan_routine(&context, &graph, &snapshot, PlanRequest::routine()).unwrap();
    assert_eq!(plan.checks()[0].selected_tool(), "dash");
    MediatorFixture {
        repo,
        context,
        graph,
        snapshot,
        plan,
    }
}

pub(crate) fn fixture(label: &str, dirty: bool) -> MediatorFixture {
    fixture_for_tool(label, dirty, "dash")
}

pub(crate) fn fixture_for_tool(label: &str, dirty: bool, tool: &str) -> MediatorFixture {
    let repo = TempRepo::new(label);
    repo.write(".git/info/exclude", b"target/\n");
    for node_id in ["syntax", "compile", "unit"] {
        std::fs::create_dir_all(repo.root().join(format!("target/routine/{node_id}"))).unwrap();
    }
    if dirty {
        repo.write("src/lib.rs", b"pub fn value() -> u8 { 59 }\n");
    }
    let context = mediator_context_for_tool(&repo, "routine-mediator", tool);
    let snapshot = LocalDirtyTree::capture(&context).unwrap();
    let graph = mediator_graph_for_tool(tool);
    let plan = plan_routine(&context, &graph, &snapshot, PlanRequest::routine()).unwrap();
    MediatorFixture {
        repo,
        context,
        graph,
        snapshot,
        plan,
    }
}

pub(crate) fn command_script(node_id: &str) -> String {
    format!(
        "printf '%s' '{node_id}' > 'target/routine/{node_id}/result.txt'; printf '{{\"schema_version\":\"RoutineCommandReport-v1\",\"request_id\":\"%s\",\"protocol_id\":\"%s\",\"intent_id\":\"%s\",\"node_id\":\"%s\",\"outcome\":\"passed\",\"behavior_observed\":true}}' \"$HUL_ROUTINE_REQUEST_ID\" \"$HUL_ROUTINE_PROTOCOL_ID\" \"$HUL_ROUTINE_INTENT_ID\" \"$HUL_ROUTINE_NODE_ID\""
    )
}

pub(crate) fn command_file_script() -> Vec<u8> {
    format!(
        "node=$HUL_ROUTINE_NODE_ID; printf '%s' \"$node\" > \"target/routine/$node/result.txt\"; {}",
        report_script()
    )
    .into_bytes()
}
