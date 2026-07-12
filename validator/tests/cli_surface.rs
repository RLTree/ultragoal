use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

static NEXT: AtomicU64 = AtomicU64::new(0);

const PRIVATE_CANARY: &str = "NEVER_ECHO_PUBLIC_DISPATCH_CANARY_8841";
const LEGACY_USAGE_MARKER: &str = "Current-state and completion evidence";

#[derive(Debug, Eq, PartialEq)]
struct SnapshotRow {
    relative_path: PathBuf,
    kind: &'static str,
    byte_length: u64,
    content_sha256: String,
    unix_mode: u32,
    modified_seconds: i64,
    modified_nanoseconds: i64,
}

#[derive(Debug, Eq, PartialEq)]
struct Observation {
    tree: Vec<SnapshotRow>,
    status: Vec<u8>,
}

struct Repository {
    root: PathBuf,
}

impl Repository {
    #[cfg(unix)]
    fn new(label: &str) -> Self {
        let live = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("validator has repository parent");
        let root = std::env::temp_dir().join(format!(
            "hul-successor-cli-surface-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).expect("create fixture root");
        let root = fs::canonicalize(root).expect("canonical fixture root");
        git(&root, &["init", "-q"]);
        git(
            &root,
            &["config", "user.email", "successor@example.invalid"],
        );
        git(&root, &["config", "user.name", "Successor CLI"]);
        git(&root, &["config", "commit.gpgsign", "false"]);
        copy_authority_inputs(live, &root);
        fs::write(root.join(".gitignore"), b"validation_artifacts/\n").expect("gitignore");
        fs::write(root.join("tracked.txt"), b"tracked baseline\n").expect("tracked file");
        git(&root, &["add", "-A"]);
        git(&root, &["commit", "-qm", "successor fixture"]);
        fs::write(root.join("tracked.txt"), b"tracked dirty state retained\n")
            .expect("dirty tracked file");
        fs::write(root.join("private-canary.txt"), PRIVATE_CANARY).expect("private canary");
        Self { root }
    }

    fn run(&self, args: &[&str]) -> Output {
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
        command.output().expect("successor CLI executes")
    }
}

impl Drop for Repository {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn copy_authority_inputs(live: &Path, root: &Path) {
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
        .expect("git executes");
    assert!(output.status.success(), "git {args:?}: {output:?}");
    output
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(unix)]
fn content(path: &Path, metadata: &fs::Metadata) -> Vec<u8> {
    if metadata.is_file() {
        fs::read(path).expect("read snapshot file")
    } else if metadata.file_type().is_symlink() {
        fs::read_link(path)
            .expect("read snapshot link")
            .as_os_str()
            .as_bytes()
            .to_vec()
    } else {
        Vec::new()
    }
}

#[cfg(unix)]
fn push_row(root: &Path, path: &Path, rows: &mut Vec<SnapshotRow>) {
    let metadata = fs::symlink_metadata(path).expect("snapshot metadata");
    let kind = if metadata.is_file() {
        "file"
    } else if metadata.is_dir() {
        "directory"
    } else if metadata.file_type().is_symlink() {
        "symlink"
    } else {
        "special"
    };
    rows.push(SnapshotRow {
        relative_path: path
            .strip_prefix(root)
            .expect("snapshot relative")
            .to_path_buf(),
        kind,
        byte_length: metadata.len(),
        content_sha256: digest(&content(path, &metadata)),
        unix_mode: metadata.mode(),
        modified_seconds: metadata.mtime(),
        modified_nanoseconds: metadata.mtime_nsec(),
    });
    if metadata.is_dir() {
        let mut entries = fs::read_dir(path)
            .expect("snapshot directory")
            .map(|entry| entry.expect("snapshot entry").path())
            .collect::<Vec<_>>();
        entries.sort();
        for entry in entries {
            push_row(root, &entry, rows);
        }
    }
}

#[cfg(unix)]
fn observe(root: &Path) -> Observation {
    let mut tree = Vec::new();
    push_row(root, root, &mut tree);
    Observation {
        tree,
        status: git(
            root,
            &["status", "--porcelain=v1", "-z", "--untracked-files=all"],
        )
        .stdout,
    }
}

fn emitted(output: &Output) -> Vec<u8> {
    let mut bytes = output.stdout.clone();
    bytes.extend_from_slice(&output.stderr);
    bytes
}

fn assert_machine_error(output: &Output) {
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let value: serde_json::Value =
        serde_json::from_slice(&emitted(output)).expect("machine error JSON");
    assert_eq!(value["schema_version"], "harness-ultragoal.cli-error.v1");
    assert_eq!(value["exit_code"], 2);
    let bytes = emitted(output);
    let text = String::from_utf8_lossy(&bytes);
    assert!(!text.contains(PRIVATE_CANARY));
    assert!(!text.contains(LEGACY_USAGE_MARKER));
}

#[cfg(unix)]
#[test]
fn public_catalog_is_sole_and_legacy_routes_fail_closed_without_writes() {
    let repository = Repository::new("sole-catalog");
    let initial = observe(&repository.root);
    assert!(!initial.status.is_empty(), "fixture must be dirty");

    for args in [
        &["--json", PRIVATE_CANARY][..],
        &["--json", "help"][..],
        &["--json", "current-state", "--help"][..],
        &["--json", "observe", "logs", "query", "--help"][..],
        &["--json", "package", "digest", "--help"][..],
        &["--json", "source", "audit"][..],
    ] {
        let before = observe(&repository.root);
        let output = repository.run(args);
        assert_machine_error(&output);
        assert_eq!(observe(&repository.root), before, "hidden write: {args:?}");
    }

    let help = repository.run(&["--json", "--help"]);
    assert_eq!(help.status.code(), Some(0));
    assert!(help.stderr.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&help.stdout).expect("help JSON");
    assert_eq!(value["schema_version"], "harness-ultragoal.cli-help.v1");
    let groups = value["commands"]
        .as_array()
        .expect("command catalog")
        .iter()
        .filter_map(|row| row["group"].as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        groups,
        BTreeSet::from([
            "check", "diagnose", "eval", "fit", "inspect", "migrate", "next", "observe", "package",
            "prove"
        ])
    );
    assert_eq!(observe(&repository.root), initial);
}

#[cfg(unix)]
#[test]
fn public_read_help_parse_and_query_paths_are_recursively_zero_write() {
    let repository = Repository::new("read-zero-write");
    let initial = observe(&repository.root);
    for args in [
        &["--json", "--version"][..],
        &["--json", "inspect", "context"][..],
        &["--json", "inspect", "inventory"][..],
        &["--json", "inspect"][..],
        &["--json", "next"][..],
        &["--json", "diagnose"][..],
        &["--json", "observe", "query"][..],
    ] {
        let before = observe(&repository.root);
        let output = repository.run(args);
        assert!(
            matches!(output.status.code(), Some(0 | 1 | 3 | 4)),
            "{args:?}: {output:?}"
        );
        let value: serde_json::Value =
            serde_json::from_slice(&emitted(&output)).expect("versioned JSON output");
        assert!(
            value["schema_version"]
                .as_str()
                .is_some_and(|schema| schema.ends_with("-v1") || schema.ends_with(".v1")),
            "{args:?}: {value}"
        );
        assert!(!String::from_utf8_lossy(&emitted(&output)).contains(PRIVATE_CANARY));
        assert_eq!(observe(&repository.root), before, "hidden write: {args:?}");
    }
    assert_eq!(observe(&repository.root), initial);
}

#[cfg(unix)]
#[test]
fn public_context_never_echoes_the_operator_supplied_absolute_root() {
    let repository = Repository::new("context-root-redaction");
    let before = observe(&repository.root);
    let output = repository.run(&["--json", "inspect", "context"]);

    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stderr.is_empty());
    assert!(!String::from_utf8_lossy(&output.stdout).contains(repository.root.to_str().unwrap()));
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["schema_version"], "HarnessPublicContext-v1");
    assert!(value["roots"]["repository_root_id"].as_str().is_some());
    assert!(value["roots"]["worktree_root_id"].as_str().is_some());
    assert!(value["roots"].get("repository_root").is_none());
    assert!(value["roots"].get("worktree_root").is_none());
    assert_eq!(observe(&repository.root), before);
}

#[cfg(unix)]
#[test]
fn unavailable_effectful_routes_refuse_before_repository_mutation_or_path_echo() {
    let repository = Repository::new("effect-refusal");
    let initial = observe(&repository.root);
    for args in [
        &[
            "--json",
            "package",
            "build",
            "--output",
            "private-output.json",
        ][..],
        &[
            "--json",
            "prove",
            "--claim",
            "private-claim",
            "--output",
            "private-proof.json",
        ][..],
        &[
            "--json",
            "migrate",
            "apply",
            "--plan",
            "private-plan.json",
            "--accept-plan",
            "private-plan-id",
        ][..],
    ] {
        let before = observe(&repository.root);
        let output = repository.run(args);
        assert_eq!(output.status.code(), Some(4), "{args:?}: {output:?}");
        let value: serde_json::Value =
            serde_json::from_slice(&emitted(&output)).expect("authority refusal JSON");
        assert_eq!(value["exit_class"], "unsupported_capability");
        let bytes = emitted(&output);
        let text = String::from_utf8_lossy(&bytes);
        for private in [
            "private-output.json",
            "private-claim",
            "private-proof.json",
            "private-plan.json",
            "private-plan-id",
        ] {
            assert!(!text.contains(private), "private argument echo: {private}");
        }
        assert_eq!(observe(&repository.root), before, "hidden write: {args:?}");
    }
    assert_eq!(observe(&repository.root), initial);
}

#[test]
fn malformed_root_options_use_the_typed_machine_contract() {
    let binary = env!("CARGO_BIN_EXE_ultragoal");
    for args in [
        &["--json", "--root"][..],
        &[
            "--json",
            "--root",
            "private-root-one",
            "--root",
            "private-root-two",
            "inspect",
        ][..],
    ] {
        let output = Command::new(binary).args(args).output().expect("CLI runs");
        assert_machine_error(&output);
        let bytes = emitted(&output);
        let text = String::from_utf8_lossy(&bytes);
        assert!(!text.contains("private-root-one"));
        assert!(!text.contains("private-root-two"));
    }
}
