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
        let fixture_root = PathBuf::from(
            std::env::var_os("CODEX_WORKTREE_ROOT")
                .expect("configured worktree root is required for public fixtures"),
        )
        .join("target/routine-public-contract-fixtures");
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
        set_mode(&home, 0o700);
        fs::write(root.join("src/lib.rs"), b"pub fn value() -> u8 { 1 }\n").unwrap();
        fs::write(root.join(".gitignore"), b"target/\n").unwrap();

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
        let binary = &self.binary;
        let mut command = Command::new(&binary);
        command
            .env_clear()
            .env("HOME", &self.home)
            .env("LC_ALL", "C")
            .env("LANG", "C")
            .env("PATH", binary.parent().unwrap())
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_OPTIONAL_LOCKS", "0")
            .env("GIT_TERMINAL_PROMPT", "0")
            .current_dir(&self.root)
            .arg("--root")
            .arg(&self.root);
        command
    }

    fn source_binary() -> PathBuf {
        std::env::var_os("HUL_ROUTINE_IMMUTABLE_BINARY")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(env!("CARGO_BIN_EXE_ultragoal")))
    }
}
