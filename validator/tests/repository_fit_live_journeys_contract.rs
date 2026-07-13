#![cfg(unix)]

use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

const BASE: &str = "/tmp/hul-repository-fit-live-journeys-067";
const PRIVATE_CANARY: &str = "REPOSITORY_FIT_PRIVATE_COMMAND_CANARY_067";
static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    id: String,
    class: String,
    expect: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadOperation {
    id: String,
    args: Vec<String>,
    schema_version: String,
    exit_code: i32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Catalog {
    schema_version: String,
    temporary_root: String,
    supported_host: String,
    claim_effect: String,
    production_permit_issuer: String,
    public_apply_dispatch: String,
    cases: Vec<Case>,
    read_operations: Vec<ReadOperation>,
}

#[derive(Debug, Eq, PartialEq)]
struct SnapshotRow {
    relative_path: PathBuf,
    kind: &'static str,
    device: u64,
    inode: u64,
    links: u64,
    uid: u32,
    gid: u32,
    unix_mode: u32,
    byte_length: u64,
    content_sha256: String,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    changed_seconds: i64,
    changed_nanoseconds: i64,
}

#[derive(Debug, Eq, PartialEq)]
struct Observation {
    tree: Vec<SnapshotRow>,
    git_status: Vec<u8>,
}

struct CommandRepository {
    container: PathBuf,
    root: PathBuf,
}

impl CommandRepository {
    fn new() -> Self {
        let container = PathBuf::from(BASE).join(format!(
            "command-zero-write-{}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        let root = container.join("repo");
        fs::create_dir_all(root.join("nested/empty")).unwrap();
        let root = fs::canonicalize(root).unwrap();
        git(&root, &["init", "--quiet"]);
        git(
            &root,
            &["config", "user.email", "fit-command@example.invalid"],
        );
        git(&root, &["config", "user.name", "Repository Fit Command"]);
        git(&root, &["config", "commit.gpgsign", "false"]);
        git(&root, &["config", "gc.auto", "0"]);
        git(&root, &["config", "maintenance.auto", "false"]);
        fs::write(root.join("tracked.txt"), b"tracked baseline\n").unwrap();
        fs::write(root.join("nested/linked.txt"), b"linked bytes\n").unwrap();
        std::os::unix::fs::symlink("linked.txt", root.join("nested/linked-symlink")).unwrap();
        git(&root, &["add", "-A"]);
        git(&root, &["commit", "--quiet", "-m", "command baseline"]);
        fs::write(root.join("tracked.txt"), b"tracked dirty user edit\n").unwrap();
        fs::write(root.join("private-canary.txt"), PRIVATE_CANARY).unwrap();
        Self { container, root }
    }

    fn run(&self, args: &[String]) -> Output {
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
            .arg("--root")
            .arg(&self.root)
            .args(args)
            .output()
            .unwrap()
    }
}

impl Drop for CommandRepository {
    fn drop(&mut self) {
        debug_assert!(self.container.starts_with(BASE));
        let _ = fs::remove_dir_all(&self.container);
    }
}

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn source(relative: &str) -> String {
    fs::read_to_string(repository_root().join(relative)).unwrap()
}

fn catalog() -> Catalog {
    serde_json::from_str(&source("fixtures/repository-fit-live-journeys/cases.json")).unwrap()
}

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn content(path: &Path, metadata: &fs::Metadata) -> Vec<u8> {
    if metadata.is_file() {
        fs::read(path).unwrap()
    } else if metadata.file_type().is_symlink() {
        fs::read_link(path).unwrap().as_os_str().as_bytes().to_vec()
    } else {
        Vec::new()
    }
}

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
    rows.push(SnapshotRow {
        relative_path: path.strip_prefix(root).unwrap().to_path_buf(),
        kind,
        device: metadata.dev(),
        inode: metadata.ino(),
        links: metadata.nlink(),
        uid: metadata.uid(),
        gid: metadata.gid(),
        unix_mode: metadata.mode(),
        byte_length: metadata.len(),
        content_sha256: digest(&content(path, &metadata)),
        modified_seconds: metadata.mtime(),
        modified_nanoseconds: metadata.mtime_nsec(),
        changed_seconds: metadata.ctime(),
        changed_nanoseconds: metadata.ctime_nsec(),
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

fn tree(root: &Path) -> Vec<SnapshotRow> {
    let mut rows = Vec::new();
    visit(root, root, &mut rows);
    rows
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

fn observe(root: &Path) -> Observation {
    let git_status = git(
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
    .stdout;
    Observation {
        tree: tree(root),
        git_status,
    }
}

#[test]
fn fixture_catalog_names_the_exact_below_root_journey_matrix() {
    let catalog = catalog();
    assert_eq!(
        catalog.schema_version,
        "RepositoryFitLiveJourneyFixtureCatalog-v1"
    );
    assert_eq!(catalog.temporary_root, BASE);
    assert_eq!(catalog.supported_host, "darwin");
    assert_eq!(catalog.claim_effect, "workspace_write");
    assert_eq!(
        catalog.production_permit_issuer,
        "darwin-owner-only-host-state"
    );
    assert_eq!(catalog.public_apply_dispatch, "successor-fit-apply");
    let observed = catalog
        .cases
        .iter()
        .map(|case| case.id.as_str())
        .collect::<BTreeSet<_>>();
    let expected = [
        "ambiguous-outcome",
        "complete-rollback",
        "conflict-refusal",
        "dirty-tree-preservation",
        "fifo-target-refusal",
        "fresh-setup",
        "inspect-command-zero-write",
        "link-and-case-alias-refusal",
        "no-effect-success-substitution",
        "partial-retrofit",
        "plan-command-zero-write",
        "public-authority-refusals",
        "public-binary-apply",
        "recovery-replan",
        "repeat-use-idempotence",
        "replay-refusal",
        "verify-as-apply-substitution",
        "verify-command-zero-write",
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    assert_eq!(observed, expected);
    assert_eq!(catalog.cases.len(), expected.len());
    assert!(
        catalog
            .cases
            .iter()
            .all(|case| !case.class.is_empty() && !case.expect.is_empty())
    );
    let read_ids = catalog
        .read_operations
        .iter()
        .map(|operation| operation.id.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        read_ids,
        [
            "inspect-command-zero-write",
            "plan-command-zero-write",
            "verify-command-zero-write",
        ]
        .into_iter()
        .collect()
    );
}

#[test]
fn direct_journeys_use_the_production_mediator_without_minting_root_authority() {
    let adapter = source("validator/src/repository_fit/product_adapter.rs");
    let tests = source("validator/src/repository_fit/product_adapter/tests.rs");
    let journeys = source("validator/src/repository_fit/product_adapter/tests/live_journeys.rs");
    let protocol = source("validator/src/repository_fit/product_adapter/protocol.rs");
    let permit = source("validator/src/repository_fit/product_adapter/root_permit.rs");
    let local = source("validator/src/repository_fit/local/effects.rs");
    assert!(tests.contains("mod live_journeys;"));
    assert!(journeys.contains("apply_with_root_permit"));
    assert!(journeys.contains("LocalEffects"));
    assert!(!journeys.contains("execute_for_test"));
    for name in [
        "positive_supported_host_fresh_setup_and_repeat_use_are_exact_and_idempotent",
        "positive_supported_host_partial_retrofit_preserves_dirty_user_state",
        "negative_conflict_and_settled_request_replay_refuse_without_effect",
        "mutation_failure_rolls_back_exactly_and_a_new_request_recovers",
        "race_after_effect_is_ambiguous_and_fresh_replanning_recovers",
        "security_link_and_case_alias_substitution_refuse_without_mutation",
        "special_file_fifo_target_refuses_before_effect_without_blocking",
        "false_pass_no_effect_success_and_verify_cannot_substitute_for_apply",
    ] {
        assert!(journeys.contains(&format!("fn {name}()")), "{name}");
    }
    assert!(protocol.contains("pub(crate) struct OpaqueFitApplyRequest"));
    assert!(permit.contains("pub(crate) trait RepositoryFitPermitEffects"));
    assert!(permit.contains("pub(crate) fn apply_with_root_permit"));
    assert!(permit.contains("#[cfg(test)]\npub(super) struct TestRepositoryFitPermitAuthority"));
    assert!(local.contains("#[cfg(test)]\n        pub(crate) fn open_for_test"));
    assert!(local.contains("mutation_lease: false"));
    assert!(!adapter.contains("pub(crate) use root_permit"));
    assert!(!adapter.contains("pub use root_permit"));
    assert!(!adapter.contains("pub(crate) mod root_permit"));
}

#[test]
fn inspect_plan_and_verify_commands_are_recursive_tree_and_git_status_zero_write() {
    let catalog = catalog();
    let repository = CommandRepository::new();
    let initial = observe(&repository.root);
    assert!(!initial.git_status.is_empty(), "fixture must remain dirty");
    for operation in catalog.read_operations {
        let before = observe(&repository.root);
        let first = repository.run(&operation.args);
        let after_first = observe(&repository.root);
        assert_eq!(after_first, before, "{} hidden write", operation.id);
        assert_eq!(
            first.status.code(),
            Some(operation.exit_code),
            "{:?}",
            first
        );
        assert!(first.stderr.is_empty(), "{}: {:?}", operation.id, first);
        let value: serde_json::Value = serde_json::from_slice(&first.stdout).unwrap();
        assert_eq!(
            value["schema_version"], operation.schema_version,
            "{}",
            operation.id
        );
        let text = String::from_utf8_lossy(&first.stdout);
        assert!(!text.contains(PRIVATE_CANARY), "{}", operation.id);
        assert!(
            !text.contains(&*repository.root.to_string_lossy()),
            "{} exposed the target path",
            operation.id
        );

        let second = repository.run(&operation.args);
        let after_second = observe(&repository.root);
        assert_eq!(after_second, before, "{} repeat hidden write", operation.id);
        assert_eq!(second.status.code(), Some(operation.exit_code));
        assert_eq!(
            second.stdout, first.stdout,
            "{} was nondeterministic",
            operation.id
        );
        assert_eq!(
            second.stderr, first.stderr,
            "{} was nondeterministic",
            operation.id
        );
    }
    assert_eq!(observe(&repository.root), initial);
}
