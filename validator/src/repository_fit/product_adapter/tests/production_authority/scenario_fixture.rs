use super::*;

pub(crate) const BASE: &str = "/private/tmp/hul-repository-fit-production-authority-085-fixtures";
pub(crate) static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

pub(crate) const AUTHORITY_SCENARIO_HELPER: &str = "repository_fit::product_adapter::tests::production_authority::subprocess_authority_scenario_helper";

pub(crate) struct ChildGuard(Option<Child>);

impl ChildGuard {
    pub(crate) fn new(child: Child) -> Self {
        Self(Some(child))
    }

    pub(crate) fn wait_with_output(mut self) -> Output {
        let child = self.0.take().expect("child is present");
        child.wait_with_output().expect("child output is available")
    }

    pub(crate) fn try_wait(&mut self) -> Option<std::process::ExitStatus> {
        self.0
            .as_mut()
            .expect("child is present")
            .try_wait()
            .expect("child status is available")
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Some(child) = self.0.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
pub(crate) struct ProofWorkerResultV1 {
    pub(crate) worker: String,
    pub(crate) lease_id: String,
    pub(crate) context_id: String,
    pub(crate) candidate_identity: BTreeMap<String, serde_json::Value>,
    pub(crate) base_state: BTreeMap<String, serde_json::Value>,
    pub(crate) final_state: BTreeMap<String, serde_json::Value>,
    pub(crate) touched_paths: Vec<String>,
    pub(crate) touched_semantics: Vec<String>,
    pub(crate) generated_outputs: Vec<String>,
    pub(crate) fixtures: Vec<String>,
    pub(crate) effects: Vec<serde_json::Value>,
    pub(crate) requirements: Vec<String>,
    pub(crate) dependency_nodes: Vec<String>,
    pub(crate) changes: Vec<BTreeMap<String, serde_json::Value>>,
    pub(crate) commands_and_tests: Vec<BTreeMap<String, serde_json::Value>>,
    pub(crate) artifacts: Vec<ProofArtifactRecord>,
    pub(crate) findings: Vec<BTreeMap<String, serde_json::Value>>,
    pub(crate) unresolved_dependencies: Vec<String>,
    pub(crate) requested_root_changes: Vec<serde_json::Value>,
    pub(crate) limitations: Vec<String>,
    pub(crate) no_claim_statement: String,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProofArtifactRecord {
    pub(crate) path: String,
    pub(crate) sha256: String,
    pub(crate) byte_length: u64,
}

pub(crate) struct Fixture {
    pub(crate) container: PathBuf,
    pub(crate) root: PathBuf,
    pub(crate) store: TestStore,
}

impl Fixture {
    pub(crate) fn new(label: &str) -> Self {
        let serial = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let container = PathBuf::from(BASE).join(format!(
            "{}-{}-{serial}",
            label.replace(|character: char| !character.is_ascii_alphanumeric(), "-"),
            std::process::id()
        ));
        let root = container.join("repo");
        let store_root = container.join("authority");
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&store_root).unwrap();
        fs::set_permissions(&store_root, fs::Permissions::from_mode(0o700)).unwrap();
        git(&root, &["init", "--quiet"]);
        Self {
            container,
            root,
            store: TestStore {
                root: store_root,
                id: digest(format!("repository-fit-store:{label}:{serial}").as_bytes()),
            },
        }
    }

    pub(crate) fn context(&self) -> LiveContext {
        LiveContext::build(BuildRequest::new(&self.root)).unwrap()
    }

    pub(crate) fn prepared(&self, context: &LiveContext) -> PreparedFitApply {
        let plan = plan_target(context).unwrap();
        prepare_apply_request(
            context,
            &plan.to_machine_bytes().unwrap(),
            plan.plan_sha256(),
        )
        .unwrap()
    }

    pub(crate) fn write(&self, relative: &str, bytes: &[u8]) {
        let path = self.root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }

    pub(crate) fn write_template(&self, relative: &str) {
        let template = CANONICAL_TEMPLATES
            .iter()
            .find(|row| row.target_path == relative)
            .unwrap();
        self.write(relative, template.bytes);
        fs::set_permissions(
            self.root.join(relative),
            fs::Permissions::from_mode(template.unix_mode),
        )
        .unwrap();
    }

    pub(crate) fn install_all_direct(&self) {
        for row in CANONICAL_TEMPLATES {
            self.write(row.target_path, row.bytes);
            fs::set_permissions(
                self.root.join(row.target_path),
                fs::Permissions::from_mode(row.unix_mode),
            )
            .unwrap();
        }
    }

    pub(crate) fn store_names(&self) -> Vec<String> {
        let mut names = fs::read_dir(&self.store.root)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        names.sort();
        names
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        debug_assert!(self.container.starts_with(BASE));
        let _ = fs::remove_dir_all(&self.container);
    }
}

pub(crate) struct TestStore {
    pub(crate) root: PathBuf,
    pub(crate) id: String,
}

impl RepositoryFitAuthorityStore for TestStore {
    fn protected_root(&self) -> &Path {
        &self.root
    }

    fn store_id(&self) -> &str {
        &self.id
    }

    fn revalidate_protected_root(&self) -> bool {
        true
    }
}

pub(crate) struct RevalidationStore<'a> {
    pub(crate) inner: &'a TestStore,
    pub(crate) calls: AtomicU64,
    pub(crate) accepted_calls: u64,
}
