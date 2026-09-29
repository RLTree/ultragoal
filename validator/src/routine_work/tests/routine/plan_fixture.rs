use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Mutex, MutexGuard};

use super::context::{BuildRequest, LiveContext};
use super::owned_compile_scratch::OwnedCompileScratch;
use super::routine_work::{
    CheckClass, DirtySnapshot, ImpactGraph, LocalDirtyTree, PathMatcher, PlanRequest,
    PreparedRoutineExecution, RepoPath, RoutineAdapterSpec, RoutineInvocationSpec, RoutinePlan,
    bind_rust_source_syntax_invocation, plan_routine, prepare_routine_execution,
};
use super::scenario::{TempRepo, node, path, route};

pub(crate) struct RoutinePlanFixture {
    pub(crate) repo: TempRepo,
    pub(crate) context: LiveContext,
    pub(crate) graph: ImpactGraph,
    pub(crate) snapshot: DirtySnapshot,
    pub(crate) plan: RoutinePlan,
    program: OwnedCompileScratch,
    _path_lock: MutexGuard<'static, ()>,
}

impl RoutinePlanFixture {
    pub(crate) fn new(label: &str) -> Self {
        let path_lock = program_path_lock().lock().unwrap();
        let program = expose_current_test_program();
        let repo = TempRepo::new(label);
        repo.write(".git/info/exclude", b"target/\n");
        repo.write("src/lib.rs", b"pub fn value() -> u8 { 9 }\n");
        fs::create_dir_all(repo.root().join("target/routine")).unwrap();
        let context = LiveContext::build(
            BuildRequest::new(repo.root())
                .bind_non_secret_configuration("profile", "routine-typed-binding")
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
            program,
            _path_lock: path_lock,
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

    pub(crate) fn finish(&mut self) {
        self.program.teardown_after_assertions();
        self.repo.teardown_after_assertions();
    }
}

pub(crate) fn repo_path(value: &str) -> RepoPath {
    path(value)
}

pub(crate) fn authority_path(fixture: &RoutinePlanFixture) -> PathBuf {
    fixture.program.path().join(".routine-authority")
}

pub(crate) fn isolate_fixture_test(test: &str) -> bool {
    const CHILD: &str = "HUL_ROUTINE_PLAN_FIXTURE_CHILD";
    if std::env::var(CHILD).ok().as_deref() == Some(test) {
        return false;
    }
    let status = Command::new(std::env::current_exe().unwrap())
        .args([test, "--exact", "--test-threads=1"])
        .env(CHILD, test)
        .status()
        .unwrap();
    assert!(status.success(), "isolated routine fixture failed: {test}");
    true
}

fn expose_current_test_program() -> OwnedCompileScratch {
    let program = OwnedCompileScratch::claim("routine-plan-program");
    let target = program.path().join("ultragoal");
    if let Some(source) = std::env::var_os("HUL_ROUTINE_IMMUTABLE_BINARY") {
        fs::copy(source, &target).unwrap();
        fs::set_permissions(&target, fs::Permissions::from_mode(0o555)).unwrap();
    } else {
        symlink(std::env::current_exe().unwrap(), &target).unwrap();
    }
    let value = std::env::join_paths(std::iter::once(program.path().to_path_buf()).chain(
        std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()),
    ))
    .unwrap();
    unsafe { std::env::set_var("PATH", value) };
    program
}

fn program_path_lock() -> &'static Mutex<()> {
    static LOCK: Mutex<()> = Mutex::new(());
    &LOCK
}
