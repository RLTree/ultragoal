use std::fs;
use std::os::unix::fs::symlink;
use std::sync::OnceLock;

use super::context::{BuildRequest, LiveContext};
use super::routine_work::{
    CheckClass, DirtySnapshot, ImpactGraph, LocalDirtyTree, PathMatcher, PlanRequest,
    RoutineInvocationSpec, RoutinePlan, bind_rust_source_syntax_invocation, plan_routine,
};
use super::scenario::{TempRepo, node, path, route};

pub(crate) struct AdapterFixture {
    pub(crate) context: LiveContext,
    pub(crate) graph: ImpactGraph,
    pub(crate) snapshot: DirtySnapshot,
    pub(crate) plan: RoutinePlan,
    _repo: TempRepo,
}

pub(crate) fn dirty_fixture(label: &str) -> AdapterFixture {
    let repo = TempRepo::new(label);
    repo.write("src/lib.rs", b"pub fn value() -> u8 { 9 }\n");
    expose_current_ultragoal();
    let context = LiveContext::build(
        BuildRequest::new(repo.root())
            .bind_non_secret_configuration("profile", "routine")
            .probe_tool("ultragoal"),
    )
    .unwrap();
    let snapshot = LocalDirtyTree::capture(&context).unwrap();
    let graph = ImpactGraph::new(
        vec![
            node("syntax", &[], CheckClass::Routine, "ultragoal", None),
            node(
                "compile",
                &["syntax"],
                CheckClass::Routine,
                "ultragoal",
                None,
            ),
            node("unit", &["compile"], CheckClass::Routine, "ultragoal", None),
        ],
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
    AdapterFixture {
        context,
        graph,
        snapshot,
        plan,
        _repo: repo,
    }
}

pub(crate) fn invocation_specs(
    context: &LiveContext,
    plan: &RoutinePlan,
) -> Vec<RoutineInvocationSpec> {
    plan.checks()
        .iter()
        .map(|check| {
            bind_rust_source_syntax_invocation(
                context,
                plan,
                check.node_id(),
                vec![path("src/lib.rs")],
                60_000,
                4 * 1024 * 1024,
                vec![path("target/routine")],
            )
            .unwrap()
        })
        .collect()
}

fn expose_current_ultragoal() {
    static PATH: OnceLock<std::ffi::OsString> = OnceLock::new();
    let path = PATH.get_or_init(|| {
        let directory =
            std::env::temp_dir().join(format!("hul-routine-binding-{}", std::process::id()));
        fs::create_dir_all(&directory).unwrap();
        let link = directory.join("ultragoal");
        let executable = std::env::current_exe().unwrap();
        if fs::read_link(&link).ok().as_ref() != Some(&executable) {
            let _ = fs::remove_file(&link);
            symlink(executable, &link).unwrap();
        }
        std::env::join_paths(std::iter::once(directory).chain(std::env::split_paths(
            &std::env::var_os("PATH").unwrap_or_default(),
        )))
        .unwrap()
    });
    unsafe { std::env::set_var("PATH", path) };
}
