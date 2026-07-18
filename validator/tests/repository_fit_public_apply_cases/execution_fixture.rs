use super::*;

pub(crate) static NEXT: AtomicU64 = AtomicU64::new(0);

pub(crate) struct Fixture {
    pub(crate) container: PathBuf,
    pub(crate) root: PathBuf,
    pub(crate) home: PathBuf,
    pub(crate) temp: PathBuf,
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
        let temp = container.join("temp");
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&temp).unwrap();
        let root = fs::canonicalize(root).unwrap();
        let home = fs::canonicalize(home).unwrap();
        let temp = fs::canonicalize(temp).unwrap();

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
            temp,
            authority,
            pending,
        }
    }

    pub(crate) fn run(&self, args: &[&str]) -> Output {
        self.command(args).output().unwrap()
    }

    pub(crate) fn command(&self, args: &[&str]) -> Command {
        let mut command = self.configured_command(env!("CARGO_BIN_EXE_ultragoal"));
        command.arg("--root").arg(&self.root).args(args);
        command
    }

    pub(crate) fn gated_command(&self, gate: &Path, args: &[&str]) -> Command {
        let mut command = self.configured_command("/bin/sh");
        command
            .arg(gate)
            .arg(env!("CARGO_BIN_EXE_ultragoal"))
            .arg("--root")
            .arg(&self.root)
            .args(args);
        command
    }

    fn configured_command(&self, program: &str) -> Command {
        let mut command = Command::new(program);
        command
            .env_clear()
            .env("HOME", &self.home)
            .env("TMPDIR", &self.temp)
            .env("LC_ALL", "C")
            .env("LANG", "C")
            .env("PATH", "/usr/bin:/bin")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_OPTIONAL_LOCKS", "0")
            .env("GIT_TERMINAL_PROMPT", "0")
            .current_dir(&self.root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        command
    }

    pub(crate) fn plan(&self) -> (String, Vec<u8>) {
        let before_root = snapshot(&self.root);
        let before_home = snapshot(&self.home);
        let before_temp = snapshot(&self.temp);
        let before_status = status(&self.root);
        let output = self.run(&["--json", "fit", "plan"]);
        assert_eq!(output.status.code(), Some(0), "{output:?}");
        assert!(output.stderr.is_empty(), "{output:?}");
        assert!(output.stdout.ends_with(b"\n"));
        assert_eq!(snapshot(&self.root), before_root);
        assert_eq!(snapshot(&self.home), before_home);
        assert_eq!(snapshot(&self.temp), before_temp);
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
        let before_temp = snapshot(&self.temp);
        let before_status = status(&self.root);
        let output = self.run(&["--json", "fit", "verify"]);
        assert_eq!(output.status.code(), Some(0), "{output:?}");
        assert!(output.stderr.is_empty(), "{output:?}");
        assert_eq!(snapshot(&self.root), before_root);
        assert_eq!(snapshot(&self.home), before_home);
        assert_eq!(snapshot(&self.temp), before_temp);
        assert_eq!(status(&self.root), before_status);
        serde_json::from_slice(&output.stdout).unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.container);
    }
}
