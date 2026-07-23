use super::*;

pub(crate) static NEXT: AtomicU64 = AtomicU64::new(0);

#[derive(Clone)]
pub(crate) struct NodeSpec {
    pub id: &'static str,
    pub dependencies: &'static [&'static str],
    pub read_sources: &'static [&'static str],
}

#[derive(Clone)]
pub(crate) struct RouteSpec {
    pub id: &'static str,
    pub kind: &'static str,
    pub path: &'static str,
    pub nodes: &'static [&'static str],
}

pub(crate) fn pass_node(id: &'static str, dependencies: &'static [&'static str]) -> NodeSpec {
    NodeSpec {
        id,
        dependencies,
        read_sources: &["src/lib.rs"],
    }
}

pub(crate) fn prefix_route(
    id: &'static str,
    path: &'static str,
    nodes: &'static [&'static str],
) -> RouteSpec {
    RouteSpec {
        id,
        kind: "prefix",
        path,
        nodes,
    }
}

pub(crate) struct Fixture {
    permit: Option<FixturePermit>,
    pub(crate) container: PathBuf,
    pub root: PathBuf,
    pub home: PathBuf,
    binary: PathBuf,
}

impl Fixture {
    pub fn new(
        label: &str,
        nodes: &[NodeSpec],
        routes: &[RouteSpec],
        dirty: bool,
        provision_host: bool,
    ) -> Self {
        let permit = Some(FixturePermit::claim());
        let fixture_root = Self::fixture_parent().join("routine-public-contract-fixtures");
        fs::create_dir_all(&fixture_root).unwrap();
        let container = fixture_root.join(format!(
            "hul-routine-public-production-102-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let root = container.join("repo");
        let home = container.join("home");
        let binary = container.join("bin/ultragoal");
        fs::create_dir_all(root.join("config")).unwrap();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(binary.parent().unwrap()).unwrap();
        fs::copy(Self::source_binary(), &binary).unwrap();
        set_mode(&binary, 0o555);
        set_mode(&home, 0o755);
        fs::write(root.join("src/lib.rs"), b"pub fn value() -> u8 { 1 }\n").unwrap();
        fs::write(root.join(".gitignore"), b"target/\nvalidation_artifacts/\n").unwrap();

        let graph = graph(nodes, routes);
        let catalog = catalog_bytes(nodes, graph.graph_id());
        fs::write(root.join("config/routines.json"), &catalog).unwrap();
        let manifest = manifest_bytes(nodes, routes, &catalog);
        fs::write(root.join("config/routine-public.json"), &manifest).unwrap();

        git(&root, &["init", "--quiet"]);
        git(
            &root,
            &["config", "user.email", "routine-public@example.invalid"],
        );
        git(&root, &["config", "user.name", "Routine Public Contract"]);
        git(&root, &["config", "commit.gpgsign", "false"]);
        git(&root, &["config", "gc.auto", "0"]);
        git(&root, &["config", "maintenance.auto", "false"]);
        git(&root, &["add", "-A"]);
        git(&root, &["commit", "--quiet", "-m", "routine fixture"]);
        if dirty {
            fs::write(root.join("src/lib.rs"), b"pub fn value() -> u8 { 2 }\n").unwrap();
        }
        if provision_host {
            provision_host_state(&home);
        }
        Self {
            permit,
            container,
            root: fs::canonicalize(root).unwrap(),
            home: fs::canonicalize(home).unwrap(),
            binary: fs::canonicalize(binary).unwrap(),
        }
    }

    pub fn run(&self) -> Output {
        self.command().output().unwrap()
    }

    pub fn run_args(&self, args: &[&str]) -> Output {
        let mut command = self.base_command();
        command.args(args).output().unwrap()
    }

    pub(crate) fn teardown_after_assertions(&mut self) {
        assert!(
            self.container.is_dir(),
            "public fixture scope disappeared before explicit teardown: {}",
            self.container.display()
        );
        fs::remove_dir_all(&self.container).expect("public fixture teardown failed");
        assert!(
            !self.container.exists(),
            "public fixture teardown retained scope: {}",
            self.container.display()
        );
        self.permit
            .take()
            .expect("fixture permit was already released");
    }

    pub fn value(output: &Output) -> Value {
        serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
            panic!(
                "stdout is not JSON: {error}; stdout={:?}; stderr={:?}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            )
        })
    }

    pub fn state_root(&self) -> PathBuf {
        self.home
            .join(".codex/state/harness-ultragoal/routine-public")
    }

    pub fn authority_root(&self) -> PathBuf {
        self.state_root().join("authority")
    }

    pub fn lock_path(&self) -> PathBuf {
        self.state_root().join("adapter/adapter.lock")
    }

    pub fn checkpoint_path(&self) -> PathBuf {
        super::continuation_paths::checkpoint_path(&self.state_root())
    }

    pub fn continuations_root(&self) -> PathBuf {
        super::continuation_paths::continuations_root(&self.state_root())
    }

    pub fn checkpoint_stage_path(&self) -> PathBuf {
        super::continuation_paths::checkpoint_stage_path(&self.state_root())
    }

    pub(crate) fn binary_path(&self) -> &Path {
        &self.binary
    }

    pub fn status(&self) -> Vec<u8> {
        git_output(
            &self.root,
            &[
                "--no-optional-locks",
                "status",
                "--porcelain=v2",
                "-z",
                "--untracked-files=all",
            ],
        )
    }

    pub fn rewrite_catalog_with_arbitrary_script(&self) {
        let catalog_path = self.root.join("config/routines.json");
        let mut catalog: Value = serde_json::from_slice(&fs::read(&catalog_path).unwrap()).unwrap();
        catalog["routines"][0]["primary"] = json!({
            "tool": "sh",
            "arguments": ["-c", "touch target/routine/compile/false-pass"]
        });
        let catalog = serde_json::to_vec(&catalog).unwrap();
        fs::write(&catalog_path, &catalog).unwrap();
        self.rebind_manifest_catalog(&catalog);
    }

    pub fn rebind_manifest_catalog(&self, catalog: &[u8]) {
        let path = self.root.join("config/routine-public.json");
        let mut manifest: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        manifest["catalog"]["sha256"] = Value::String(sha(catalog));
        manifest["catalog"]["byte_length"] = Value::from(catalog.len() as u64);
        fs::write(path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    }

    pub fn downgrade_manifest_schema(&self) {
        let path = self.root.join("config/routine-public.json");
        let mut manifest: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        manifest["schema_version"] = Value::String("RoutinePublicProduction-v1".to_owned());
        fs::write(path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    }

    pub fn substitute_lock_with_symlink(&self) {
        let lock = self.lock_path();
        fs::remove_file(&lock).unwrap();
        let substitute = self.state_root().join("substitute.lock");
        fs::write(&substitute, b"routine-public-lock-v1\n").unwrap();
        set_mode(&substitute, 0o600);
        symlink(&substitute, lock).unwrap();
    }

    pub(crate) fn command(&self) -> Command {
        let mut command = self.base_command();
        command.args(["--json", "check", "routine"]);
        command
    }

    pub(crate) fn base_command(&self) -> Command {
        routine_command(&self.root, &self.home, &self.binary)
    }

    fn source_binary() -> PathBuf {
        std::env::var_os("HUL_ROUTINE_IMMUTABLE_BINARY")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(env!("CARGO_BIN_EXE_ultragoal")))
    }

    fn fixture_parent() -> PathBuf {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let workspace = manifest
            .parent()
            .expect("public fixture manifest has no workspace parent");
        workspace.join("target")
    }
}
