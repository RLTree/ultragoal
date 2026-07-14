use std::fs;
use std::os::unix::fs::symlink;
use std::path::PathBuf;
use std::sync::OnceLock;

use super::context::{BuildRequest, LiveContext};
use super::routine_work::{
    CheckClass, DirtySnapshot, ImpactGraph, LocalDirtyTree, PathMatcher, PlanRequest,
    PreparedRoutineExecution, RepoPath, RoutineAdapterSpec, RoutineInvocationSpec, RoutinePlan,
    bind_rust_source_syntax_invocation, plan_routine, prepare_routine_execution,
};
use super::scenario::{TempRepo, node, path, route};

pub(crate) struct CurrentPathFixture {
    pub(crate) repo: TempRepo,
    pub(crate) context: LiveContext,
    pub(crate) graph: ImpactGraph,
    pub(crate) snapshot: DirtySnapshot,
    pub(crate) plan: RoutinePlan,
}

impl CurrentPathFixture {
    pub(crate) fn new(label: &str) -> Self {
        expose_current_test_program();
        let repo = TempRepo::new(label);
        repo.write(".git/info/exclude", b"target/\n");
        repo.write("src/lib.rs", b"pub fn value() -> u8 { 9 }\n");
        fs::create_dir_all(repo.root().join("target/routine")).unwrap();
        let context = LiveContext::build(
            BuildRequest::new(repo.root())
                .bind_non_secret_configuration("profile", "routine-current-path")
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
        Self {
            repo,
            context,
            graph,
            snapshot,
            plan,
        }
    }

    pub(crate) fn invocations(&self) -> Vec<RoutineInvocationSpec> {
        self.plan
            .checks()
            .iter()
            .map(|check| self.invocation(check.node_id()))
            .collect()
    }

    pub(crate) fn invocation(&self, node_id: &str) -> RoutineInvocationSpec {
        bind_rust_source_syntax_invocation(
            &self.context,
            &self.plan,
            node_id,
            vec![path("src/lib.rs")],
            60_000,
            4 * 1024 * 1024,
            vec![path("target/routine")],
        )
        .unwrap()
    }

    pub(crate) fn prepare(
        &self,
    ) -> Result<PreparedRoutineExecution, super::routine_work::RoutineError> {
        self.prepare_with(self.invocations())
    }

    pub(crate) fn prepare_with(
        &self,
        invocations: Vec<RoutineInvocationSpec>,
    ) -> Result<PreparedRoutineExecution, super::routine_work::RoutineError> {
        prepare_routine_execution(
            &self.context,
            &self.graph,
            &self.snapshot,
            &self.plan,
            RoutineAdapterSpec::new("routine", invocations),
        )
    }
}

pub(crate) fn repo_path(value: &str) -> RepoPath {
    path(value)
}

pub(crate) fn authority_path(fixture: &CurrentPathFixture) -> PathBuf {
    fixture.repo.root().join(".routine-authority")
}

fn expose_current_test_program() {
    static PATH_VALUE: OnceLock<std::ffi::OsString> = OnceLock::new();
    let value = PATH_VALUE.get_or_init(|| {
        let directory = std::env::temp_dir().join(format!(
            "hul-routine-current-path-program-{}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        let link = directory.join("ultragoal");
        let target = std::env::current_exe().unwrap();
        if fs::read_link(&link).ok().as_deref() != Some(target.as_path()) {
            let _ = fs::remove_file(&link);
            symlink(target, &link).unwrap();
        }
        std::env::join_paths(std::iter::once(directory).chain(std::env::split_paths(
            &std::env::var_os("PATH").unwrap_or_default(),
        )))
        .unwrap()
    });
    unsafe { std::env::set_var("PATH", value) };
}
