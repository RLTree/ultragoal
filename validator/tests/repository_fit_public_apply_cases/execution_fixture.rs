use super::*;

pub(crate) static NEXT: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct SnapshotRow {
    pub(crate) path: PathBuf,
    pub(crate) kind: &'static str,
    pub(crate) device: u64,
    pub(crate) inode: u64,
    pub(crate) links: u64,
    pub(crate) uid: u32,
    pub(crate) gid: u32,
    pub(crate) mode: u32,
    pub(crate) size: u64,
    pub(crate) modified_seconds: i64,
    pub(crate) modified_nanoseconds: i64,
    pub(crate) changed_seconds: i64,
    pub(crate) changed_nanoseconds: i64,
    pub(crate) content_sha256: String,
}

pub(crate) struct Fixture {
    pub(crate) container: PathBuf,
    pub(crate) root: PathBuf,
    pub(crate) home: PathBuf,
    pub(crate) authority: PathBuf,
    pub(crate) pending: PathBuf,
}

impl Fixture {
    pub(crate) fn new(label: &str) -> Self {
        let live = repository_root();
        let container = std::env::temp_dir().join(format!(
            "hul-repository-fit-public-apply-091-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let root = container.join("repo");
        let home = container.join("home");
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&home).unwrap();
        let root = fs::canonicalize(root).unwrap();
        let home = fs::canonicalize(home).unwrap();

        git(&root, &["init", "--quiet"]);
        git(
            &root,
            &["config", "user.email", "public-fit@example.invalid"],
        );
        git(&root, &["config", "user.name", "Public Fit Contract"]);
        git(&root, &["config", "commit.gpgsign", "false"]);
        git(&root, &["config", "gc.auto", "0"]);
        git(&root, &["config", "maintenance.auto", "false"]);
        copy_authority_inputs(&live, &root);
        git(&root, &["add", "-A"]);
        git(&root, &["commit", "--quiet", "-m", "public fit fixture"]);

        let state = home.join(".codex/state/harness-ultragoal/repository-fit");
        let authority = state.join("authority");
        let pending = state.join("pending");
        fs::create_dir_all(&authority).unwrap();
        fs::create_dir_all(&pending).unwrap();
        for path in [
            &home,
            &home.join(".codex"),
            &home.join(".codex/state"),
            &home.join(".codex/state/harness-ultragoal"),
            &state,
            &authority,
            &pending,
        ] {
            fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
        }

        Self {
            container,
            root,
            home,
            authority,
            pending,
        }
    }

    pub(crate) fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_ultragoal"))
            .env_clear()
            .env("HOME", &self.home)
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
            .args(args)
            .output()
            .unwrap()
    }

    pub(crate) fn plan(&self) -> (String, Vec<u8>) {
        let before_root = snapshot(&self.root);
        let before_home = snapshot(&self.home);
        let before_status = status(&self.root);
        let output = self.run(&["--json", "fit", "plan"]);
        assert_eq!(output.status.code(), Some(0), "{output:?}");
        assert!(output.stderr.is_empty(), "{output:?}");
        assert!(output.stdout.ends_with(b"\n"));
        assert_eq!(snapshot(&self.root), before_root);
        assert_eq!(snapshot(&self.home), before_home);
        assert_eq!(status(&self.root), before_status);
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        let plan_sha256 = value["plan"]["plan_sha256"].as_str().unwrap().to_owned();
        fs::create_dir_all(self.root.join("validation_artifacts")).unwrap();
        fs::write(
            self.root.join("validation_artifacts/fit-plan.json"),
            &output.stdout,
        )
        .unwrap();
        (plan_sha256, output.stdout)
    }

    pub(crate) fn apply(&self, plan_sha256: &str) -> Output {
        self.run(&[
            "--json",
            "fit",
            "apply",
            "--plan",
            "validation_artifacts/fit-plan.json",
            "--accept-plan",
            plan_sha256,
        ])
    }

    pub(crate) fn verify_zero_write(&self) -> Value {
        let before_root = snapshot(&self.root);
        let before_home = snapshot(&self.home);
        let before_status = status(&self.root);
        let output = self.run(&["--json", "fit", "verify"]);
        assert_eq!(output.status.code(), Some(0), "{output:?}");
        assert!(output.stderr.is_empty(), "{output:?}");
        assert_eq!(snapshot(&self.root), before_root);
        assert_eq!(snapshot(&self.home), before_home);
        assert_eq!(status(&self.root), before_status);
        serde_json::from_slice(&output.stdout).unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.container);
    }
}

#[test]
pub(crate) fn public_binary_applies_verifies_and_repeats_through_durable_authority() {
    let fixture = Fixture::new("positive");
    let (plan_sha256, _) = fixture.plan();
    let applied = fixture.apply(&plan_sha256);
    assert_eq!(applied.status.code(), Some(0), "{applied:?}");
    assert!(applied.stderr.is_empty(), "{applied:?}");
    let value: Value = serde_json::from_slice(&applied.stdout).unwrap();
    assert_eq!(value["schema_version"], "RepositoryFitProductionOutcome-v1");
    assert_eq!(value["status"], "applied");
    assert_eq!(value["effect"], "workspace_write");
    assert_eq!(
        fs::read(fixture.root.join("AGENTS.md")).unwrap(),
        fs::read(repository_root().join("templates/AGENTS.md")).unwrap()
    );
    assert_eq!(fs::read_dir(&fixture.authority).unwrap().count(), 3);
    assert_eq!(pending_entries(&fixture.pending).len(), 1);
    assert!(pending_entries(&fixture.pending)[0].ends_with(".lock"));

    let verified = fixture.verify_zero_write();
    assert_eq!(verified["schema_version"], "RepositoryFitVerification-v1");
    assert_eq!(verified["idempotent"], true);

    let (repeat_plan_sha256, _) = fixture.plan();
    let before_repeat = snapshot(&fixture.root);
    let repeated = fixture.apply(&repeat_plan_sha256);
    assert_eq!(repeated.status.code(), Some(0), "{repeated:?}");
    assert!(repeated.stderr.is_empty(), "{repeated:?}");
    let value: Value = serde_json::from_slice(&repeated.stdout).unwrap();
    assert_eq!(value["status"], "idempotent");
    assert_eq!(snapshot(&fixture.root), before_repeat);
    assert_eq!(pending_entries(&fixture.pending).len(), 1);
    assert!(pending_entries(&fixture.pending)[0].ends_with(".lock"));
}
