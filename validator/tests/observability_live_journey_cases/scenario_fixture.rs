use super::*;

pub(crate) const BASE: &str = ".git/codex-scratch/observability/local-diagnosis-069";
pub(crate) const SOURCE_ID: &str = "successor-runtime";
pub(crate) const PRIVATE_TOKEN: &str = "sk-observability-private-canary-069";
pub(crate) const PRIVATE_PATH: &str = "/Users/private/observability-canary-069";
pub(crate) const PRIVATE_EMAIL: &str = "private-observability-069@example.invalid";
pub(crate) const STORE_RELATIVE: &str =
    "validation_artifacts/observability/spool/successor-events.jsonl";
pub(crate) static NEXT: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Case {
    pub(crate) id: String,
    pub(crate) class: String,
    pub(crate) expect: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Catalog {
    pub(crate) schema_version: String,
    pub(crate) temporary_root: String,
    pub(crate) source_id: String,
    pub(crate) claim_effect: String,
    pub(crate) external_export_default: String,
    pub(crate) cases: Vec<Case>,
}

#[derive(Clone, Debug)]
pub(crate) struct Binding {
    pub(crate) context_id: String,
    pub(crate) candidate_id: String,
    pub(crate) source_id: String,
}

#[derive(Clone, Debug)]
pub(crate) struct SelectedFinding {
    pub(crate) finding_id: String,
    pub(crate) repair_id: String,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct SnapshotRow {
    pub(crate) relative_path: PathBuf,
    pub(crate) kind: &'static str,
    pub(crate) device: u64,
    pub(crate) inode: u64,
    pub(crate) links: u64,
    pub(crate) uid: u32,
    pub(crate) gid: u32,
    pub(crate) unix_mode: u32,
    pub(crate) byte_length: u64,
    pub(crate) content_sha256: String,
    pub(crate) modified_seconds: i64,
    pub(crate) modified_nanoseconds: i64,
    pub(crate) changed_seconds: i64,
    pub(crate) changed_nanoseconds: i64,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct Observation {
    pub(crate) tree: Vec<SnapshotRow>,
    pub(crate) git_status: Vec<u8>,
}

pub(crate) struct JourneyRepository {
    pub(crate) container: PathBuf,
    pub(crate) root: PathBuf,
}

impl JourneyRepository {
    pub(crate) fn new(label: &str, legacy_conflict: bool, dirty: bool) -> Self {
        let container = live_root().join(BASE).join(format!(
            "{}-{}-{}",
            safe_label(label),
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let root = container.join("repo");
        fs::create_dir_all(&root).expect("create journey repository");
        let root = fs::canonicalize(root).expect("canonical journey repository");
        let repository = Self { container, root };
        repository.git(&["init", "--quiet"]);
        repository.git(&["config", "user.email", "observability-069@example.invalid"]);
        repository.git(&["config", "user.name", "Observability Journey"]);
        repository.git(&["config", "commit.gpgsign", "false"]);
        repository.git(&["config", "gc.auto", "0"]);
        repository.git(&["config", "maintenance.auto", "false"]);
        repository.git(&["config", "maintenance.autoDetach", "false"]);
        copy_authority_inputs(repository.root());
        fs::write(
            repository.root.join(".gitignore"),
            b"validation_artifacts/\n",
        )
        .expect("write fixture ignore");
        fs::write(
            repository.root.join("tracked.txt"),
            b"tracked observability baseline\n",
        )
        .expect("write tracked baseline");
        if legacy_conflict {
            let conflict = repository.root.join("legacy/command-catalog.json");
            fs::create_dir_all(conflict.parent().expect("conflict parent"))
                .expect("create conflict parent");
            fs::write(conflict, b"{}\n").expect("write legacy conflict");
        }
        repository.git(&["add", "-A"]);
        repository.git(&["commit", "--quiet", "-m", "observability fixture"]);
        if dirty {
            fs::write(
                repository.root.join("tracked.txt"),
                b"dirty tracked observability bytes retained\n",
            )
            .expect("write dirty tracked fixture");
            fs::write(repository.root.join("private-canary.txt"), PRIVATE_TOKEN)
                .expect("write private untracked fixture");
        }
        repository
    }

    pub(crate) fn root(&self) -> &Path {
        &self.root
    }

    pub(crate) fn store_path(&self) -> PathBuf {
        self.root.join(STORE_RELATIVE)
    }

    pub(crate) fn outside(&self, name: &str) -> PathBuf {
        self.container.join(name)
    }

    pub(crate) fn run(&self, args: &[&str]) -> Output {
        self.run_with_env(args, &[])
    }

    pub(crate) fn run_with_env(&self, args: &[&str], environment: &[(&str, &str)]) -> Output {
        let command = self.command_with_env(args, environment);
        output_with_timeout(command, Duration::from_secs(60))
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

    pub(crate) fn git(&self, args: &[&str]) -> Output {
        let mut command = Command::new("/usr/bin/git");
        command
            .args(args)
            .env_clear()
            .env("LC_ALL", "C")
            .env("LANG", "C")
            .env("PATH", "/usr/bin:/bin")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_OPTIONAL_LOCKS", "0")
            .env("GIT_TERMINAL_PROMPT", "0")
            .current_dir(&self.root);
        let output = output_with_timeout(command, Duration::from_secs(10));
        assert!(output.status.success(), "git {args:?}: {output:?}");
        output
    }
}

impl Drop for JourneyRepository {
    fn drop(&mut self) {
        debug_assert!(self.container.starts_with(live_root().join(BASE)));
        let _ = fs::remove_dir_all(&self.container);
    }
}

pub(crate) fn live_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("validator repository parent")
        .to_path_buf()
}

pub(crate) fn catalog() -> Catalog {
    serde_json::from_slice(
        &fs::read(live_root().join("fixtures/observability-local-diagnosis/cases.json"))
            .expect("read observability journey catalog"),
    )
    .expect("parse observability journey catalog")
}

pub(crate) fn copy_authority_inputs(root: &Path) {
    let live = live_root();
    let source = live.join("docs/ultragoal-contract-2026-07-successor-v2/FINAL-CONTRACT");
    let target = root.join("docs/ultragoal-contract-2026-07-successor-v2/FINAL-CONTRACT");
    fs::create_dir_all(&target).expect("contract target");
    let mut files = fs::read_dir(&source)
        .expect("contract directory")
        .map(|entry| entry.expect("contract entry").path())
        .filter(|path| path.is_file())
        .collect::<Vec<_>>();
    files.sort();
    for path in files {
        fs::copy(&path, target.join(path.file_name().expect("contract name")))
            .expect("copy contract file");
    }
    for name in ["FINAL-HANDOFF-MANIFEST.sha256", "README.md"] {
        fs::copy(
            source.parent().expect("contract parent").join(name),
            target.parent().expect("target parent").join(name),
        )
        .expect("copy handoff input");
    }
    fs::create_dir_all(root.join("migration")).expect("migration directory");
    for name in ["authority-routes.json", "generated-surface-authority.json"] {
        fs::copy(
            live.join("migration").join(name),
            root.join("migration").join(name),
        )
        .expect("copy migration input");
    }
    fs::create_dir_all(root.join(".codex-plugin")).expect("plugin directory");
    fs::write(
        root.join(".codex-plugin/plugin.json"),
        b"{\"name\":\"harness-ultragoal\",\"version\":\"0.0.0-test\"}\n",
    )
    .expect("plugin descriptor");
}
