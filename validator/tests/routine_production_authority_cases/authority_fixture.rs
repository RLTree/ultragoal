use super::*;

pub(crate) static NEXT_AUTHORITY: AtomicU64 = AtomicU64::new(1);

pub(crate) struct Fixture {
    pub(crate) repo: TempRepo,
    pub(crate) context: LiveContext,
    pub(crate) graph: ImpactGraph,
    pub(crate) snapshot: DirtySnapshot,
    pub(crate) plan: RoutinePlan,
}

pub(crate) struct AuthorityRoot {
    pub(crate) parent: PathBuf,
    pub(crate) path: PathBuf,
}

impl AuthorityRoot {
    pub(crate) fn new(label: &str) -> Self {
        let sequence = NEXT_AUTHORITY.fetch_add(1, Ordering::Relaxed);
        let parent = std::env::temp_dir().join(format!(
            "hul-routine-production-{label}-{}-{sequence}",
            std::process::id()
        ));
        let path = parent.join("authority");
        fs::create_dir_all(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        Self { parent, path }
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn tree(&self) -> BTreeMap<String, String> {
        tree(&self.path)
    }
}

impl Drop for AuthorityRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.parent);
    }
}

pub(crate) fn fixture(label: &str, dirty: bool) -> Fixture {
    let repo = TempRepo::new(label);
    repo.write(".git/info/exclude", b"target/\n");
    fs::create_dir_all(repo.root().join("target/routine/compile")).unwrap();
    if dirty {
        repo.write("src/lib.rs", b"pub fn value() -> u8 { 89 }\n");
    }
    fixture_from_repo(repo, "routine-production")
}

pub(crate) fn fixture_from_repo(repo: TempRepo, profile: &str) -> Fixture {
    let context = LiveContext::build(
        BuildRequest::new(repo.root())
            .bind_non_secret_configuration("profile", profile)
            .probe_tool("sandbox-exec")
            .probe_tool("dash"),
    )
    .unwrap();
    let snapshot = LocalDirtyTree::capture(&context).unwrap();
    let graph = graph();
    let plan = plan_routine(&context, &graph, &snapshot, PlanRequest::routine()).unwrap();
    Fixture {
        repo,
        context,
        graph,
        snapshot,
        plan,
    }
}

pub(crate) fn graph() -> ImpactGraph {
    ImpactGraph::new(
        vec![node("compile", &[], CheckClass::Routine, "dash", None)],
        vec![route(
            "route-src",
            PathMatcher::Prefix(path("src")),
            &["compile"],
            false,
        )],
        Vec::new(),
    )
    .unwrap()
}

pub(crate) fn prepared(fixture: &Fixture) -> PreparedRoutineExecution {
    let invocations = fixture
        .plan
        .checks()
        .iter()
        .map(|check| {
            bind_routine_invocation(
                &fixture.context,
                &fixture.plan,
                check.node_id(),
                vec!["-c".to_owned(), command_script(check.node_id())],
                10_000,
                1024 * 1024,
                vec![path(&format!("target/routine/{}", check.node_id()))],
            )
            .unwrap()
        })
        .collect();
    prepare_routine_execution(
        &fixture.context,
        &fixture.graph,
        &fixture.snapshot,
        &fixture.plan,
        RoutineAdapterSpec::new("routine", invocations),
    )
    .unwrap()
}

pub(crate) fn prepared_failure(fixture: &Fixture) -> PreparedRoutineExecution {
    let invocations = fixture
        .plan
        .checks()
        .iter()
        .map(|check| {
            bind_routine_invocation(
                &fixture.context,
                &fixture.plan,
                check.node_id(),
                vec!["-c".to_owned(), "exit 7".to_owned()],
                10_000,
                1024 * 1024,
                vec![path(&format!("target/routine/{}", check.node_id()))],
            )
            .unwrap()
        })
        .collect();
    prepare_routine_execution(
        &fixture.context,
        &fixture.graph,
        &fixture.snapshot,
        &fixture.plan,
        RoutineAdapterSpec::new("routine", invocations),
    )
    .unwrap()
}

pub(crate) fn command_script(node_id: &str) -> String {
    format!("printf '%s' '{node_id}' > 'target/routine/{node_id}/result.txt'")
}

pub(crate) fn mediate(
    authority: &AuthorityRoot,
    fixture: &Fixture,
    prepared: PreparedRoutineExecution,
    reuse: Vec<Vec<u8>>,
) -> Result<RoutineMediationResult, routine_work::RoutineError> {
    mediate_prepared_routine_execution_production(
        authority.path(),
        &fixture.context,
        &fixture.plan,
        prepared,
        None,
        RoutineCancellation::new(),
        RoutineReuseInput::new(reuse),
    )
}

pub(crate) fn authority_state(authority: &AuthorityRoot) -> Vec<u8> {
    fs::read(authority.path().join("routine-authority.state")).unwrap()
}

pub(crate) fn authority_cardinalities(authority: &AuthorityRoot) -> (usize, usize, usize) {
    let state: serde_json::Value = serde_json::from_slice(&authority_state(authority)).unwrap();
    let payload = &state["payload"];
    (
        payload["protocols"].as_object().unwrap().len(),
        payload["effects"].as_object().unwrap().len(),
        payload["consumed_grants"].as_array().unwrap().len(),
    )
}

pub(crate) fn foreign_reuse_for_same_request(fixture: &Fixture, label: &str) -> Vec<Vec<u8>> {
    let authority = AuthorityRoot::new(label);
    mediate(&authority, fixture, prepared(fixture), Vec::new())
        .unwrap()
        .reuse_artifacts()
        .to_vec()
}

pub(crate) fn corrupt_reuse_witness(bytes: &[u8]) -> Vec<u8> {
    let mut forged = String::from_utf8(bytes.to_vec()).unwrap();
    let wire: serde_json::Value = serde_json::from_str(&forged).unwrap();
    let witness = wire["mediator_witness_sha256"].as_str().unwrap();
    forged = forged.replace(
        witness,
        "sha256:0000000000000000000000000000000000000000000000000000000000000000",
    );
    forged.into_bytes()
}
