use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, Deserialize)]
pub(super) struct JourneyCase {
    pub(super) id: String,
    pub(super) dirty: bool,
    pub(super) legacy_conflict: bool,
    pub(super) expected_finding_code: Option<String>,
}

#[derive(Deserialize)]
struct JourneyCatalog {
    schema_version: String,
    cases: Vec<JourneyCase>,
}

pub(super) fn journey_cases() -> Vec<JourneyCase> {
    let path = live_root().join("fixtures/successor-cli-state-journeys/cases.json");
    let catalog: JourneyCatalog =
        serde_json::from_slice(&fs::read(path).expect("read journey catalog"))
            .expect("parse journey catalog");
    assert_eq!(
        catalog.schema_version,
        "SuccessorCliStateJourneyFixtures-v1"
    );
    assert_eq!(catalog.cases.len(), 3);
    catalog.cases
}

pub(super) struct JourneyRepository {
    root: PathBuf,
}

impl JourneyRepository {
    pub(super) fn new(case: &JourneyCase) -> Self {
        let root = std::env::temp_dir().join(format!(
            "hul-successor-cli-state-047-{}-{}-{}",
            case.id,
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).expect("create journey root");
        let root = fs::canonicalize(root).expect("canonical journey root");
        let repository = Self { root };
        repository.git(&["init", "-q"]);
        repository.git(&["config", "user.email", "successor-047@example.invalid"]);
        repository.git(&["config", "user.name", "Successor Read Journey"]);
        repository.git(&["config", "commit.gpgsign", "false"]);
        repository.git(&["config", "gc.auto", "0"]);
        repository.git(&["config", "maintenance.auto", "false"]);
        repository.git(&["config", "maintenance.autoDetach", "false"]);
        copy_authority_inputs(repository.root());
        fs::write(
            repository.root.join(".gitignore"),
            b"validation_artifacts/\n",
        )
        .expect("write gitignore");
        fs::write(repository.root.join("tracked.txt"), b"tracked baseline\n")
            .expect("write tracked fixture");
        if case.legacy_conflict {
            let path = repository.root.join("legacy/command-catalog.json");
            fs::create_dir_all(path.parent().expect("legacy parent")).expect("legacy dir");
            fs::write(path, b"{}\n").expect("legacy authority conflict");
        }
        repository.git(&["add", "-A"]);
        repository.git(&["commit", "-qm", "successor read journey fixture"]);
        if case.dirty {
            fs::write(
                repository.root.join("tracked.txt"),
                b"dirty tracked bytes retained\n",
            )
            .expect("dirty tracked fixture");
            fs::write(
                repository.root.join("untracked-private-canary.txt"),
                b"SUCCESSOR_READ_PRIVATE_CANARY_047\n",
            )
            .expect("dirty untracked fixture");
        }
        repository
    }

    pub(super) fn root(&self) -> &Path {
        &self.root
    }

    pub(super) fn run(&self, args: &[&str]) -> Output {
        let mut all = vec!["--root".to_owned(), self.root.display().to_string()];
        all.extend(args.iter().map(|value| (*value).to_owned()));
        self.execute(&all)
    }

    pub(super) fn run_raw(&self, args: &[&str]) -> Output {
        self.execute(
            &args
                .iter()
                .map(|value| (*value).to_owned())
                .collect::<Vec<_>>(),
        )
    }

    fn execute(&self, args: &[String]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_ultragoal"))
            .env_clear()
            .env("LC_ALL", "C")
            .env("LANG", "C")
            .env("PATH", "/usr/bin:/bin")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_OPTIONAL_LOCKS", "0")
            .env("GIT_TERMINAL_PROMPT", "0")
            .current_dir(&self.root)
            .args(args)
            .output()
            .expect("fresh successor binary executes")
    }

    fn git(&self, args: &[&str]) {
        let output = Command::new("/usr/bin/git")
            .args(args)
            .env_clear()
            .env("LC_ALL", "C")
            .env("LANG", "C")
            .env("PATH", "/usr/bin:/bin")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_OPTIONAL_LOCKS", "0")
            .env("GIT_TERMINAL_PROMPT", "0")
            .current_dir(&self.root)
            .output()
            .expect("git fixture command");
        assert!(output.status.success(), "git {args:?}: {output:?}");
    }
}

impl Drop for JourneyRepository {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn live_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("validator repository parent")
        .to_path_buf()
}

fn copy_authority_inputs(root: &Path) {
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
