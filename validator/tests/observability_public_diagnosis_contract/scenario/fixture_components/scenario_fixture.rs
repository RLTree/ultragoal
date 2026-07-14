use super::*;

pub(crate) const BASE: &str = ".git/codex-scratch/observability/public-diagnosis-107";
pub(crate) const SOURCE_ID: &str = "successor-runtime";
pub(crate) const PRIVATE_TOKEN: &str = "sk-public-diagnosis-private-107";
pub(crate) const PRIVATE_PATH: &str = "/Users/private/public-diagnosis-107";
pub(crate) const PRIVATE_EMAIL: &str = "public-diagnosis-107@example.invalid";
pub(crate) const STORE: &str = "validation_artifacts/observability/spool/successor-events.jsonl";
pub(crate) static NEXT: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug)]
pub(crate) struct Binding {
    pub context_id: String,
    pub candidate_id: String,
    pub source_id: String,
}

#[derive(Clone, Debug)]
pub(crate) struct SelectedFinding {
    pub finding_id: String,
    pub repair_id: String,
}

pub(crate) struct Repository {
    pub(crate) container: PathBuf,
    pub(crate) root: PathBuf,
}

impl Repository {
    pub(crate) fn new(label: &str, conflict: bool, dirty: bool) -> Self {
        let container = live_root().join(BASE).join(format!(
            "{}-{}-{}",
            safe_label(label),
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let root = container.join("repo");
        fs::create_dir_all(&root).unwrap();
        let root = fs::canonicalize(root).unwrap();
        let repository = Self { container, root };
        repository.git(&["init", "--quiet"]);
        repository.git(&["config", "user.email", "public-diagnosis@example.invalid"]);
        repository.git(&["config", "user.name", "Public Diagnosis"]);
        repository.git(&["config", "commit.gpgsign", "false"]);
        repository.git(&["config", "gc.auto", "0"]);
        repository.git(&["config", "maintenance.auto", "false"]);
        copy_authority_inputs(repository.root());
        fs::write(
            repository.root.join(".gitignore"),
            b"validation_artifacts/\n",
        )
        .unwrap();
        fs::write(
            repository.root.join("tracked.txt"),
            b"public diagnosis baseline\n",
        )
        .unwrap();
        if conflict {
            let path = repository.root.join("legacy/command-catalog.json");
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, b"{}\n").unwrap();
        }
        repository.git(&["add", "-A"]);
        repository.git(&["commit", "--quiet", "-m", "fixture"]);
        if dirty {
            fs::write(
                repository.root.join("tracked.txt"),
                b"dirty retained bytes\n",
            )
            .unwrap();
            fs::write(repository.root.join("private-canary.txt"), PRIVATE_TOKEN).unwrap();
        }
        repository
    }

    pub(crate) fn root(&self) -> &Path {
        &self.root
    }

    pub(crate) fn store_path(&self) -> PathBuf {
        self.root.join(STORE)
    }

    pub(crate) fn outside(&self, name: &str) -> PathBuf {
        self.container.join(name)
    }

    pub(crate) fn run(&self, args: &[&str]) -> Output {
        self.run_with_env(args, &[])
    }

    pub(crate) fn run_with_env(&self, args: &[&str], environment: &[(&str, &str)]) -> Output {
        output_with_timeout(
            self.command_with_env(args, environment),
            Duration::from_secs(60),
        )
    }

    pub(crate) fn command_with_env(&self, args: &[&str], environment: &[(&str, &str)]) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_ultragoal"));
        command
            .env_clear()
            .env("LC_ALL", "C")
            .env("LANG", "C")
            .env("PATH", "/usr/bin:/bin")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_OPTIONAL_LOCKS", "0")
            .env("GIT_TERMINAL_PROMPT", "0")
            .current_dir(&self.root)
            .arg("--root")
            .arg(&self.root)
            .args(args);
        for (key, value) in environment {
            command.env(key, value);
        }
        command
    }

    pub(crate) fn git(&self, args: &[&str]) {
        let output = Command::new("/usr/bin/git")
            .args(args)
            .env_clear()
            .env("LC_ALL", "C")
            .env("LANG", "C")
            .env("PATH", "/usr/bin:/bin")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_OPTIONAL_LOCKS", "0")
            .current_dir(&self.root)
            .output()
            .unwrap();
        assert!(output.status.success(), "git {args:?}: {output:?}");
    }
}

impl Drop for Repository {
    fn drop(&mut self) {
        debug_assert!(self.container.starts_with(live_root().join(BASE)));
        let _ = fs::remove_dir_all(&self.container);
    }
}

pub(crate) fn live_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

pub(crate) fn open_store(repository: &Repository, binding: &Binding) -> EventStore {
    fs::create_dir_all(repository.store_path().parent().unwrap()).unwrap();
    EventStore::open_bound(
        repository.store_path(),
        &binding.context_id,
        &binding.candidate_id,
        &binding.source_id,
    )
    .unwrap()
}

pub(crate) fn event(
    binding: &Binding,
    id: &str,
    sequence: u64,
    operation: &str,
    outcome: &str,
) -> SemanticEvent {
    SemanticEvent::new(
        &binding.context_id,
        &binding.candidate_id,
        &binding.source_id,
        id,
        sequence,
        sequence,
        operation,
        outcome,
    )
    .unwrap()
}

pub(crate) fn copy_authority_inputs(root: &Path) {
    let live = live_root();
    let source = live.join("docs/ultragoal-contract-2026-07-successor-v2/FINAL-CONTRACT");
    let target = root.join("docs/ultragoal-contract-2026-07-successor-v2/FINAL-CONTRACT");
    fs::create_dir_all(&target).unwrap();
    let mut files = fs::read_dir(&source)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_file())
        .collect::<Vec<_>>();
    files.sort();
    for path in files {
        fs::copy(&path, target.join(path.file_name().unwrap())).unwrap();
    }
    for name in ["FINAL-HANDOFF-MANIFEST.sha256", "README.md"] {
        fs::copy(
            source.parent().unwrap().join(name),
            target.parent().unwrap().join(name),
        )
        .unwrap();
    }
    fs::create_dir_all(root.join("migration")).unwrap();
    for name in ["authority-routes.json", "generated-surface-authority.json"] {
        fs::copy(
            live.join("migration").join(name),
            root.join("migration").join(name),
        )
        .unwrap();
    }
    fs::create_dir_all(root.join(".codex-plugin")).unwrap();
    fs::write(
        root.join(".codex-plugin/plugin.json"),
        b"{\"name\":\"harness-ultragoal\",\"version\":\"0.0.0-test\"}\n",
    )
    .unwrap();
}
