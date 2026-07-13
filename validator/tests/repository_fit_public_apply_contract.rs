#![cfg(target_vendor = "apple")]

use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Eq, PartialEq)]
struct SnapshotRow {
    path: PathBuf,
    kind: &'static str,
    device: u64,
    inode: u64,
    links: u64,
    uid: u32,
    gid: u32,
    mode: u32,
    size: u64,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    changed_seconds: i64,
    changed_nanoseconds: i64,
    content_sha256: String,
}

struct Fixture {
    container: PathBuf,
    root: PathBuf,
    home: PathBuf,
    authority: PathBuf,
    pending: PathBuf,
}

impl Fixture {
    fn new(label: &str) -> Self {
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

    fn run(&self, args: &[&str]) -> Output {
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

    fn plan(&self) -> (String, Vec<u8>) {
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

    fn apply(&self, plan_sha256: &str) -> Output {
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

    fn verify_zero_write(&self) -> Value {
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
fn public_binary_applies_verifies_and_repeats_through_durable_authority() {
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

#[test]
fn public_binary_refuses_acceptance_framing_and_host_substitution_without_effect() {
    let fixture = Fixture::new("refusals");
    let (plan_sha256, plan_bytes) = fixture.plan();
    let bad_digest = format!("sha256:{}", "0".repeat(64));

    let before_root = snapshot(&fixture.root);
    let before_home = snapshot(&fixture.home);
    let mismatch = fixture.apply(&bad_digest);
    assert_diagnostic(
        &mismatch,
        3,
        "successor_runtime_authority_required",
        &fixture,
    );
    assert_eq!(snapshot(&fixture.root), before_root);
    assert_eq!(snapshot(&fixture.home), before_home);
    assert!(!fixture.root.join("AGENTS.md").exists());

    OpenOptions::new()
        .append(true)
        .open(fixture.root.join("validation_artifacts/fit-plan.json"))
        .unwrap()
        .write_all(b"\n")
        .unwrap();
    let before_root = snapshot(&fixture.root);
    let before_home = snapshot(&fixture.home);
    let alternate_framing = fixture.apply(&plan_sha256);
    assert_diagnostic(
        &alternate_framing,
        2,
        "successor_runtime_unexpected_arguments",
        &fixture,
    );
    assert_eq!(snapshot(&fixture.root), before_root);
    assert_eq!(snapshot(&fixture.home), before_home);

    fs::write(
        fixture.root.join("validation_artifacts/fit-plan.json"),
        plan_bytes,
    )
    .unwrap();
    fs::remove_dir(&fixture.pending).unwrap();
    let substituted = fixture.container.join("substituted-pending");
    fs::create_dir(&substituted).unwrap();
    fs::set_permissions(&substituted, fs::Permissions::from_mode(0o700)).unwrap();
    symlink(&substituted, &fixture.pending).unwrap();
    let before_root = snapshot(&fixture.root);
    let before_home = snapshot(&fixture.home);
    let refusal = fixture.apply(&plan_sha256);
    assert_diagnostic(
        &refusal,
        3,
        "successor_runtime_authority_required",
        &fixture,
    );
    assert_eq!(snapshot(&fixture.root), before_root);
    assert_eq!(snapshot(&fixture.home), before_home);
    assert!(!fixture.root.join("AGENTS.md").exists());
}

#[test]
fn public_binary_rejects_a_stale_plan_before_opening_host_authority() {
    let fixture = Fixture::new("stale-plan");
    let (plan_sha256, _) = fixture.plan();
    fs::write(
        fixture.root.join("private-user-change.txt"),
        b"preserve me\n",
    )
    .unwrap();
    let before_root = snapshot(&fixture.root);
    let before_home = snapshot(&fixture.home);
    let before_status = status(&fixture.root);
    let stale = fixture.apply(&plan_sha256);
    assert_diagnostic(&stale, 1, "successor_runtime_stale_context", &fixture);
    assert_eq!(snapshot(&fixture.root), before_root);
    assert_eq!(snapshot(&fixture.home), before_home);
    assert_eq!(status(&fixture.root), before_status);
    assert!(!fixture.root.join("AGENTS.md").exists());
}

fn assert_diagnostic(output: &Output, code: i32, id: &str, fixture: &Fixture) {
    assert_eq!(output.status.code(), Some(code), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    let value: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(value["schema_version"], "HarnessDiagnostic-v1");
    assert_eq!(value["diagnostic_id"], id);
    let text = String::from_utf8_lossy(&output.stderr);
    assert!(!text.contains(&*fixture.root.to_string_lossy()));
    assert!(!text.contains(&*fixture.home.to_string_lossy()));
}

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn copy_authority_inputs(live: &Path, root: &Path) {
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
        br#"{"name":"harness-ultragoal","version":"0.0.0-test"}"#,
    )
    .unwrap();
    fs::write(root.join(".gitignore"), b"validation_artifacts/\n").unwrap();
}

fn git(root: &Path, args: &[&str]) -> Output {
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
        .current_dir(root)
        .output()
        .unwrap();
    assert!(output.status.success(), "git {args:?}: {output:?}");
    output
}

fn status(root: &Path) -> Vec<u8> {
    git(
        root,
        &[
            "-c",
            "core.fsmonitor=false",
            "-c",
            "core.untrackedCache=false",
            "status",
            "--porcelain=v1",
            "-z",
            "--untracked-files=all",
        ],
    )
    .stdout
}

fn snapshot(root: &Path) -> Vec<SnapshotRow> {
    fn visit(root: &Path, path: &Path, rows: &mut Vec<SnapshotRow>) {
        let metadata = fs::symlink_metadata(path).unwrap();
        let kind = if metadata.is_file() {
            "file"
        } else if metadata.is_dir() {
            "directory"
        } else if metadata.file_type().is_symlink() {
            "symlink"
        } else {
            "special"
        };
        let content = if metadata.is_file() {
            fs::read(path).unwrap()
        } else if metadata.file_type().is_symlink() {
            fs::read_link(path).unwrap().as_os_str().as_bytes().to_vec()
        } else {
            Vec::new()
        };
        rows.push(SnapshotRow {
            path: path.strip_prefix(root).unwrap().to_path_buf(),
            kind,
            device: metadata.dev(),
            inode: metadata.ino(),
            links: metadata.nlink(),
            uid: metadata.uid(),
            gid: metadata.gid(),
            mode: metadata.mode(),
            size: metadata.size(),
            modified_seconds: metadata.mtime(),
            modified_nanoseconds: metadata.mtime_nsec(),
            changed_seconds: metadata.ctime(),
            changed_nanoseconds: metadata.ctime_nsec(),
            content_sha256: format!("sha256:{:x}", Sha256::digest(content)),
        });
        if metadata.is_dir() {
            let mut entries = fs::read_dir(path)
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .collect::<Vec<_>>();
            entries.sort();
            for entry in entries {
                visit(root, &entry, rows);
            }
        }
    }
    let mut rows = Vec::new();
    visit(root, root, &mut rows);
    rows
}

fn pending_entries(pending: &Path) -> Vec<String> {
    let mut names = fs::read_dir(pending)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    names.sort();
    names
}
