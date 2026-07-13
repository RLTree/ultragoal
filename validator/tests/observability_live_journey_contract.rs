#![cfg(unix)]

use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::ffi::CString;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};
use ultragoal::observability::{EventQuery, EventStore, SemanticEvent};

const BASE: &str = "/tmp/hul-observability-local-diagnosis-069";
const SOURCE_ID: &str = "successor-runtime";
const PRIVATE_TOKEN: &str = "sk-observability-private-canary-069";
const PRIVATE_PATH: &str = "/Users/private/observability-canary-069";
const PRIVATE_EMAIL: &str = "private-observability-069@example.invalid";
const STORE_RELATIVE: &str = "validation_artifacts/observability/spool/successor-events.jsonl";
static NEXT: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    id: String,
    class: String,
    expect: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Catalog {
    schema_version: String,
    temporary_root: String,
    source_id: String,
    claim_effect: String,
    external_export_default: String,
    cases: Vec<Case>,
}

#[derive(Clone, Debug)]
struct Binding {
    context_id: String,
    candidate_id: String,
    source_id: String,
}

#[derive(Clone, Debug)]
struct SelectedFinding {
    finding_id: String,
    repair_id: String,
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

struct JourneyRepository {
    container: PathBuf,
    root: PathBuf,
}

impl JourneyRepository {
    fn new(label: &str, legacy_conflict: bool, dirty: bool) -> Self {
        let container = PathBuf::from(BASE).join(format!(
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

    fn root(&self) -> &Path {
        &self.root
    }

    fn store_path(&self) -> PathBuf {
        self.root.join(STORE_RELATIVE)
    }

    fn outside(&self, name: &str) -> PathBuf {
        self.container.join(name)
    }

    fn run(&self, args: &[&str]) -> Output {
        self.run_with_env(args, &[])
    }

    fn run_with_env(&self, args: &[&str], environment: &[(&str, &str)]) -> Output {
        let command = self.command_with_env(args, environment);
        output_with_timeout(command, Duration::from_secs(60))
    }

    fn command_with_env(&self, args: &[&str], environment: &[(&str, &str)]) -> Command {
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

    fn git(&self, args: &[&str]) -> Output {
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
        debug_assert!(self.container.starts_with(BASE));
        let _ = fs::remove_dir_all(&self.container);
    }
}

fn live_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("validator repository parent")
        .to_path_buf()
}

fn catalog() -> Catalog {
    serde_json::from_slice(
        &fs::read(live_root().join("fixtures/observability-local-diagnosis/cases.json"))
            .expect("read observability journey catalog"),
    )
    .expect("parse observability journey catalog")
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

fn output_with_timeout(mut command: Command, timeout: Duration) -> Output {
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn source-built ultragoal");
    let mut stdout = child.stdout.take().expect("captured stdout");
    let stdout_reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout.read_to_end(&mut bytes).expect("read stdout");
        bytes
    });
    let mut stderr = child.stderr.take().expect("captured stderr");
    let stderr_reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        stderr.read_to_end(&mut bytes).expect("read stderr");
        bytes
    });
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait().expect("poll source-built ultragoal") {
            let stdout = stdout_reader.join().expect("join stdout reader");
            let stderr = stderr_reader.join().expect("join stderr reader");
            return Output {
                status,
                stdout,
                stderr,
            };
        }
        if started.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            let _ = stdout_reader.join();
            let _ = stderr_reader.join();
            panic!("source-built ultragoal exceeded {timeout:?}");
        }
        thread::sleep(Duration::from_millis(5));
    }
}

fn git_status(root: &Path) -> Vec<u8> {
    let output = Command::new("/usr/bin/git")
        .args([
            "-c",
            "core.fsmonitor=false",
            "-c",
            "core.untrackedCache=false",
            "status",
            "--porcelain=v1",
            "-z",
            "--untracked-files=all",
        ])
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
        .expect("git status observation");
    assert!(output.status.success(), "git status: {output:?}");
    output.stdout
}

fn content(path: &Path, metadata: &fs::Metadata) -> Vec<u8> {
    if metadata.is_file() {
        fs::read(path).expect("read snapshot file")
    } else if metadata.file_type().is_symlink() {
        fs::read_link(path)
            .expect("read snapshot symlink")
            .as_os_str()
            .as_bytes()
            .to_vec()
    } else {
        Vec::new()
    }
}

fn visit(root: &Path, path: &Path, rows: &mut Vec<SnapshotRow>) {
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
            .expect("snapshot prefix")
            .to_path_buf(),
        kind,
        device: metadata.dev(),
        inode: metadata.ino(),
        links: metadata.nlink(),
        uid: metadata.uid(),
        gid: metadata.gid(),
        unix_mode: metadata.mode(),
        byte_length: metadata.len(),
        content_sha256: format!("sha256:{:x}", Sha256::digest(content(path, &metadata))),
        modified_seconds: metadata.mtime(),
        modified_nanoseconds: metadata.mtime_nsec(),
        changed_seconds: metadata.ctime(),
        changed_nanoseconds: metadata.ctime_nsec(),
    });
    if metadata.is_dir() {
        let mut entries = fs::read_dir(path)
            .expect("snapshot directory")
            .map(|entry| entry.expect("snapshot entry").path())
            .collect::<Vec<_>>();
        entries.sort();
        for entry in entries {
            visit(root, &entry, rows);
        }
    }
}

fn observe(root: &Path) -> Observation {
    let git_status = git_status(root);
    let mut tree = Vec::new();
    visit(root, root, &mut tree);
    Observation { tree, git_status }
}

fn machine(bytes: &[u8]) -> Value {
    serde_json::from_slice(bytes).expect("machine JSON")
}

fn assert_private_absent(output: &Output, root: &Path) {
    let mut bytes = output.stdout.clone();
    bytes.extend_from_slice(&output.stderr);
    let text = String::from_utf8_lossy(&bytes);
    let root_text = root.to_string_lossy();
    for private in [
        PRIVATE_TOKEN,
        PRIVATE_PATH,
        PRIVATE_EMAIL,
        root_text.as_ref(),
    ] {
        assert!(!text.contains(private), "private output: {text}");
    }
}

fn assert_payload_repeat_zero_write(
    repository: &JourneyRepository,
    args: &[&str],
    expected_exits: &[i32],
    schema: &str,
) -> Value {
    let before = observe(repository.root());
    let first = repository.run(args);
    assert!(
        expected_exits.contains(&first.status.code().unwrap_or(-1)),
        "{args:?}: {first:?}"
    );
    assert!(first.stderr.is_empty(), "{args:?}: {first:?}");
    assert_private_absent(&first, repository.root());
    let value = machine(&first.stdout);
    assert_eq!(value["schema_version"], schema, "{args:?}");
    assert_eq!(observe(repository.root()), before, "hidden write: {args:?}");

    let second = repository.run(args);
    assert_eq!(second.status.code(), first.status.code(), "{args:?}");
    assert_eq!(
        second.stdout, first.stdout,
        "nondeterministic stdout: {args:?}"
    );
    assert_eq!(
        second.stderr, first.stderr,
        "nondeterministic stderr: {args:?}"
    );
    assert_eq!(
        observe(repository.root()),
        before,
        "repeat hidden write: {args:?}"
    );
    value
}

fn assert_diagnostic_zero_write(
    repository: &JourneyRepository,
    args: &[&str],
    expected_exit: i32,
    diagnostic_id: &str,
) -> Value {
    let before = observe(repository.root());
    let output = repository.run(args);
    assert_eq!(output.status.code(), Some(expected_exit), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    assert_private_absent(&output, repository.root());
    let value = machine(&output.stderr);
    assert_eq!(value["schema_version"], "HarnessDiagnostic-v1");
    assert_eq!(value["diagnostic_id"], diagnostic_id);
    assert_eq!(
        observe(repository.root()),
        before,
        "hidden diagnostic write"
    );
    value
}

fn public_binding(repository: &JourneyRepository) -> Binding {
    let value = assert_payload_repeat_zero_write(
        repository,
        &["--json", "observe", "query"],
        &[0],
        "ObservabilityQuery-v1",
    );
    assert_eq!(value["store_status"], "absent");
    Binding {
        context_id: value["context_id"]
            .as_str()
            .expect("query context id")
            .to_owned(),
        candidate_id: value["candidate_id"]
            .as_str()
            .expect("query candidate id")
            .to_owned(),
        source_id: value["source_id"]
            .as_str()
            .expect("query source id")
            .to_owned(),
    }
}

fn selected_finding(repository: &JourneyRepository) -> SelectedFinding {
    let value = assert_payload_repeat_zero_write(
        repository,
        &["--json", "inspect", "findings"],
        &[1],
        "ProductStateFindings-v1",
    );
    let row = value["findings"]
        .as_array()
        .and_then(|rows| rows.iter().find(|row| row["code"] == "parallel_authority"))
        .expect("parallel authority finding");
    SelectedFinding {
        finding_id: row["finding_id"].as_str().expect("finding id").to_owned(),
        repair_id: row["repair"]["repair_id"]
            .as_str()
            .expect("repair id")
            .to_owned(),
    }
}

fn open_store(repository: &JourneyRepository, binding: &Binding) -> EventStore {
    fs::create_dir_all(
        repository
            .store_path()
            .parent()
            .expect("observability spool parent"),
    )
    .expect("create observability spool");
    EventStore::open_bound(
        repository.store_path(),
        &binding.context_id,
        &binding.candidate_id,
        &binding.source_id,
    )
    .expect("open candidate-bound event store")
}

fn event(binding: &Binding, id: &str, sequence: u64, operation: &str) -> SemanticEvent {
    SemanticEvent::new(
        &binding.context_id,
        &binding.candidate_id,
        &binding.source_id,
        id,
        sequence,
        sequence,
        operation,
        "fail",
    )
    .expect("candidate-bound semantic event")
}

fn query(binding: &Binding) -> EventQuery {
    EventQuery::new(
        &binding.context_id,
        &binding.candidate_id,
        &binding.source_id,
    )
    .expect("candidate-bound query")
}

fn assert_no_connection(listener: &TcpListener) {
    listener
        .set_nonblocking(true)
        .expect("nonblocking network canary");
    match listener.accept() {
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
        Ok((_, address)) => panic!("unexpected disabled-export connection from {address}"),
        Err(error) => panic!("network canary failed: {error}"),
    }
}

fn safe_label(label: &str) -> String {
    label
        .chars()
        .filter(|character| character.is_ascii_alphanumeric() || *character == '-')
        .collect()
}

#[test]
fn fixture_catalog_closes_the_required_behavior_classes() {
    let catalog = catalog();
    assert_eq!(
        catalog.schema_version,
        "ObservabilityLocalDiagnosisFixtures-v1"
    );
    assert_eq!(catalog.temporary_root, BASE);
    assert_eq!(catalog.source_id, SOURCE_ID);
    assert_eq!(catalog.claim_effect, "none");
    assert_eq!(
        catalog.external_export_default,
        "disabled-safe-default-OD-004-OD-007"
    );
    let ids = catalog
        .cases
        .iter()
        .map(|case| case.id.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(ids.len(), catalog.cases.len(), "duplicate fixture id");
    let classes = catalog
        .cases
        .iter()
        .map(|case| case.class.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        classes,
        [
            "false-pass",
            "mutation",
            "negative",
            "network",
            "positive",
            "privacy",
            "race",
            "read",
            "security",
            "stale-candidate",
            "unknown-row",
        ]
        .into_iter()
        .collect()
    );
    assert!(catalog.cases.iter().all(|case| !case.expect.is_empty()));
}

#[test]
fn fresh_binary_absent_help_query_diagnose_and_export_refusal_are_zero_write() {
    let repository = JourneyRepository::new("absent-read", true, true);
    let initial = observe(repository.root());
    assert!(!initial.git_status.is_empty(), "fixture must remain dirty");

    assert_payload_repeat_zero_write(
        &repository,
        &["--json", "--help"],
        &[0],
        "harness-ultragoal.cli-help.v1",
    );
    let binding = public_binding(&repository);
    assert_eq!(binding.source_id, SOURCE_ID);
    let finding = selected_finding(&repository);
    let diagnosis = assert_payload_repeat_zero_write(
        &repository,
        &["--json", "diagnose", "--finding", &finding.finding_id],
        &[1],
        "ProductStateDiagnose-v1",
    );
    assert_eq!(diagnosis["observability"]["store_status"], "absent");
    assert_eq!(
        diagnosis["observability"]["explanation"]["classification"],
        "missing-evidence"
    );
    assert_eq!(diagnosis["claim_effect"], "none");

    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind network canary");
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let environment = [
        ("HTTP_PROXY", endpoint.as_str()),
        ("HTTPS_PROXY", endpoint.as_str()),
        ("ALL_PROXY", endpoint.as_str()),
        ("OTEL_EXPORTER_OTLP_ENDPOINT", endpoint.as_str()),
        ("ULTRAGOAL_OBSERVABILITY_EXPORT_ENDPOINT", endpoint.as_str()),
    ];
    let before_export = observe(repository.root());
    let export = repository.run_with_env(
        &[
            "--json",
            "observe",
            "export",
            "--output",
            "journey-export.json",
            "--approve-export",
        ],
        &environment,
    );
    assert_eq!(export.status.code(), Some(3), "{export:?}");
    assert!(export.stdout.is_empty(), "{export:?}");
    assert_eq!(
        machine(&export.stderr)["diagnostic_id"],
        "successor_runtime_authority_required"
    );
    assert_private_absent(&export, repository.root());
    assert_eq!(observe(repository.root()), before_export);
    assert!(!repository.root().join("journey-export.json").exists());
    assert_no_connection(&listener);
    assert_eq!(observe(repository.root()), initial);
}

#[test]
fn public_query_and_diagnosis_preserve_redacted_stable_causal_correlation() {
    let repository = JourneyRepository::new("causal", true, true);
    let binding = public_binding(&repository);
    let finding = selected_finding(&repository);
    let store = open_store(&repository, &binding);

    let root = event(&binding, "journey-root", 1, "command.prepare");
    let mut target = event(&binding, "journey-target", 2, "command.execute");
    target.set_parent(root.event_id()).unwrap();
    target.add_finding_ref(&finding.finding_id).unwrap();
    target.add_repair_ref(&finding.repair_id).unwrap();
    target
        .add_public_attribute("api_token", PRIVATE_TOKEN)
        .unwrap();
    target
        .add_public_attribute("private_path", PRIVATE_PATH)
        .unwrap();
    target.add_public_attribute("owner", PRIVATE_EMAIL).unwrap();
    target
        .add_public_attribute("safe", "bounded-public-value")
        .unwrap();
    assert!(store.append(&root).unwrap());
    assert!(store.append(&target).unwrap());
    assert!(
        !store.append(&target).unwrap(),
        "exact append must be idempotent"
    );

    let persisted = fs::read_to_string(repository.store_path()).unwrap();
    for private in [PRIVATE_TOKEN, PRIVATE_PATH, PRIVATE_EMAIL] {
        assert!(!persisted.contains(private), "persisted {private}");
    }
    assert!(persisted.contains("bounded-public-value"));

    let query_value = assert_payload_repeat_zero_write(
        &repository,
        &["--json", "observe", "query", "--filter", "command.execute"],
        &[0],
        "ObservabilityQuery-v1",
    );
    assert_eq!(query_value["context_id"], binding.context_id);
    assert_eq!(query_value["candidate_id"], binding.candidate_id);
    assert_eq!(query_value["source_id"], binding.source_id);
    assert_eq!(query_value["event_count"], 1);
    assert_eq!(query_value["events"][0]["event_id"], "journey-target");
    assert_eq!(query_value["events"][0]["parent_event_id"], "journey-root");
    assert_eq!(query_value["claim_effect"], "none");
    assert_eq!(
        query_value["local_policy"]["external_export"],
        "disabled-safe-default-OD-004-OD-007"
    );
    assert_eq!(
        query_value["local_policy"]["store_limit_bytes"],
        EventStore::supported_store_limit_bytes()
    );
    assert_eq!(
        query_value["local_policy"]["event_limit"],
        EventStore::supported_event_limit()
    );
    assert_eq!(
        query_value["local_policy"]["scan_row_limit"],
        EventStore::supported_scan_limit()
    );
    assert_eq!(
        query_value["local_policy"]["query_result_limit"],
        EventStore::supported_result_limit()
    );
    assert_eq!(
        query_value["local_policy"]["lock_timeout_millis"],
        EventStore::supported_lock_timeout_millis()
    );

    let diagnosis = assert_payload_repeat_zero_write(
        &repository,
        &["--json", "diagnose", "--finding", &finding.finding_id],
        &[1],
        "ProductStateDiagnose-v1",
    );
    assert_eq!(diagnosis["observability"]["store_status"], "available");
    assert_eq!(
        diagnosis["observability"]["matched_event_id"],
        "journey-target"
    );
    assert_eq!(
        diagnosis["observability"]["explanation"]["classification"],
        "observed-cause"
    );
    assert_eq!(
        diagnosis["observability"]["explanation"]["causal_event_ids"],
        serde_json::json!(["journey-root", "journey-target"])
    );
    assert!(
        diagnosis["observability"]["explanation"]["repair"]
            .as_str()
            .is_some_and(|repair| repair.contains("Apply the repair"))
    );
    assert!(diagnosis["repairs"].as_array().is_some_and(|repairs| {
        repairs
            .iter()
            .any(|row| row["repair_id"].as_str() == Some(finding.repair_id.as_str()))
    }));
    assert_eq!(diagnosis["claim_effect"], "none");
}

#[test]
fn receipt_only_event_cannot_false_pass_as_a_public_cause() {
    let repository = JourneyRepository::new("receipt-only", true, false);
    let binding = public_binding(&repository);
    let finding = selected_finding(&repository);
    let store = open_store(&repository, &binding);
    let mut receipt = event(&binding, "receipt-only-event", 1, "receipt.command");
    receipt.add_finding_ref(&finding.finding_id).unwrap();
    receipt.add_repair_ref(&finding.repair_id).unwrap();
    assert!(store.append(&receipt).unwrap());

    let diagnosis = assert_payload_repeat_zero_write(
        &repository,
        &["--json", "diagnose", "--finding", &finding.finding_id],
        &[1],
        "ProductStateDiagnose-v1",
    );
    assert_eq!(
        diagnosis["observability"]["matched_event_id"],
        "receipt-only-event"
    );
    assert_eq!(
        diagnosis["observability"]["explanation"]["classification"],
        "missing-evidence"
    );
    assert_eq!(
        diagnosis["observability"]["explanation"]["diagnostic_code"],
        "observe-evidence-missing:receipt-only"
    );
    assert_eq!(diagnosis["claim_effect"], "none");
}

#[test]
fn stale_unknown_and_truncated_stores_fail_closed_and_only_explicit_recovery_writes() {
    let stale = JourneyRepository::new("stale-candidate", false, false);
    let stale_binding = public_binding(&stale);
    let stale_store = open_store(&stale, &stale_binding);
    assert!(
        stale_store
            .append(&event(&stale_binding, "stale-event", 1, "check.run"))
            .unwrap()
    );
    fs::write(stale.root().join("tracked.txt"), b"new candidate bytes\n").unwrap();
    assert_diagnostic_zero_write(
        &stale,
        &["--json", "observe", "query"],
        4,
        "successor_runtime_observability_unavailable",
    );

    let unknown = JourneyRepository::new("unknown-row", true, false);
    let unknown_binding = public_binding(&unknown);
    let unknown_finding = selected_finding(&unknown);
    let unknown_store = open_store(&unknown, &unknown_binding);
    assert!(
        unknown_store
            .append(&event(
                &unknown_binding,
                "unknown-row-event",
                1,
                "check.run"
            ))
            .unwrap()
    );
    let row = fs::read_to_string(unknown.store_path()).unwrap();
    fs::write(
        unknown.store_path(),
        row.replacen("}\n", ",\"unknown_row\":true}\n", 1),
    )
    .unwrap();
    assert_diagnostic_zero_write(
        &unknown,
        &["--json", "observe", "query"],
        4,
        "successor_runtime_observability_unavailable",
    );
    let corrupt_diagnosis = assert_payload_repeat_zero_write(
        &unknown,
        &[
            "--json",
            "diagnose",
            "--finding",
            &unknown_finding.finding_id,
        ],
        &[1],
        "ProductStateDiagnose-v1",
    );
    assert_eq!(
        corrupt_diagnosis["observability"]["explanation"]["classification"],
        "corruption"
    );

    let recovery = JourneyRepository::new("truncated-recovery", false, false);
    let recovery_binding = public_binding(&recovery);
    let recovery_store = open_store(&recovery, &recovery_binding);
    assert!(
        recovery_store
            .append(&event(&recovery_binding, "retained-event", 1, "check.run"))
            .unwrap()
    );
    OpenOptions::new()
        .append(true)
        .open(recovery.store_path())
        .unwrap()
        .write_all(b"{\"row_version\":\"SemanticEventRow-v1\"")
        .unwrap();
    assert_diagnostic_zero_write(
        &recovery,
        &["--json", "observe", "query"],
        4,
        "successor_runtime_observability_unavailable",
    );
    let status_before_recovery = git_status(recovery.root());
    let removed = recovery_store.recover_truncated_tail().unwrap();
    assert!(removed > 0);
    assert_eq!(git_status(recovery.root()), status_before_recovery);
    assert_eq!(recovery_store.recover_truncated_tail().unwrap(), 0);
    let recovered = assert_payload_repeat_zero_write(
        &recovery,
        &["--json", "observe", "query"],
        &[0],
        "ObservabilityQuery-v1",
    );
    assert_eq!(recovered["event_count"], 1);
    assert_eq!(recovered["events"][0]["event_id"], "retained-event");
}

#[test]
fn bound_race_symlink_and_fifo_substitution_refuse_without_hidden_effects() {
    let raced = JourneyRepository::new("bound-race", false, false);
    let binding = public_binding(&raced);
    let store = open_store(&raced, &binding);
    assert!(
        store
            .append(&event(&binding, "race-event", 1, "check.run"))
            .unwrap()
    );
    let original = raced
        .store_path()
        .with_file_name("successor-events.original.jsonl");
    fs::rename(raced.store_path(), &original).unwrap();
    fs::write(raced.store_path(), b"").unwrap();
    let before_race_query = observe(raced.root());
    let error = store.query(&query(&binding)).unwrap_err();
    assert!(
        error.contains("identity")
            || error.contains("changed")
            || error.contains("replaced")
            || error.contains("substitution"),
        "unexpected race error: {error}"
    );
    assert_eq!(observe(raced.root()), before_race_query);
    assert_eq!(fs::read(raced.store_path()).unwrap(), b"");
    assert!(!fs::read(&original).unwrap().is_empty());

    let linked = JourneyRepository::new("symlink-parent", false, false);
    let outside = linked.outside("outside-spool");
    fs::create_dir_all(linked.root().join("validation_artifacts/observability")).unwrap();
    fs::create_dir_all(&outside).unwrap();
    symlink(
        &outside,
        linked
            .root()
            .join("validation_artifacts/observability/spool"),
    )
    .unwrap();
    let outside_before = fs::read_dir(&outside).unwrap().count();
    assert_diagnostic_zero_write(
        &linked,
        &["--json", "observe", "query"],
        4,
        "successor_runtime_observability_unavailable",
    );
    assert_eq!(fs::read_dir(&outside).unwrap().count(), outside_before);

    let fifo = JourneyRepository::new("fifo-leaf", false, false);
    fs::create_dir_all(fifo.store_path().parent().unwrap()).unwrap();
    let fifo_name = CString::new(fifo.store_path().as_os_str().as_bytes()).unwrap();
    let result = unsafe { libc::mkfifo(fifo_name.as_ptr(), 0o600) };
    assert_eq!(
        result,
        0,
        "mkfifo failed: {}",
        std::io::Error::last_os_error()
    );
    assert_diagnostic_zero_write(
        &fifo,
        &["--json", "observe", "query"],
        4,
        "successor_runtime_observability_unavailable",
    );
}

#[test]
fn source_built_query_exits_at_the_lock_deadline_with_deterministic_zero_write_output() {
    let repository = JourneyRepository::new("exclusive-lock-deadline", true, true);
    let binding = public_binding(&repository);
    let store = open_store(&repository, &binding);
    assert!(
        store
            .append(&event(&binding, "lock-deadline-event", 1, "check.run"))
            .unwrap()
    );
    let holder = OpenOptions::new()
        .read(true)
        .write(true)
        .open(repository.store_path())
        .unwrap();
    holder.lock().unwrap();

    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind network canary");
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let environment = [
        ("HTTP_PROXY", endpoint.as_str()),
        ("HTTPS_PROXY", endpoint.as_str()),
        ("ALL_PROXY", endpoint.as_str()),
        ("OTEL_EXPORTER_OTLP_ENDPOINT", endpoint.as_str()),
        ("ULTRAGOAL_OBSERVABILITY_EXPORT_ENDPOINT", endpoint.as_str()),
    ];
    let before = observe(repository.root());
    let timeout = Duration::from_millis(EventStore::supported_lock_timeout_millis());

    let first_started = Instant::now();
    let first = repository.run_with_env(&["--json", "observe", "query"], &environment);
    assert_public_lock_timeout(&first, repository.root(), first_started.elapsed(), timeout);
    assert_eq!(
        observe(repository.root()),
        before,
        "contended query wrote state"
    );
    assert_no_connection(&listener);

    let mut interrupted_command =
        repository.command_with_env(&["--json", "observe", "query"], &environment);
    let mut interrupted = interrupted_command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn interruptible source-built query");
    thread::sleep(Duration::from_millis(250));
    assert!(
        interrupted.try_wait().unwrap().is_none(),
        "contended query did not remain in its bounded wait"
    );
    interrupted.kill().unwrap();
    let _ = interrupted.wait().unwrap();
    assert_eq!(
        observe(repository.root()),
        before,
        "interrupted contention wrote state"
    );
    assert_no_connection(&listener);

    let second_started = Instant::now();
    let second = repository.run_with_env(&["--json", "observe", "query"], &environment);
    assert_public_lock_timeout(
        &second,
        repository.root(),
        second_started.elapsed(),
        timeout,
    );
    assert_eq!(second.status.code(), first.status.code());
    assert_eq!(second.stdout, first.stdout, "contended stdout changed");
    assert_eq!(second.stderr, first.stderr, "contended diagnostic changed");
    assert_eq!(
        observe(repository.root()),
        before,
        "repeat contention wrote state"
    );
    assert_no_connection(&listener);

    holder.unlock().unwrap();
    let released = assert_payload_repeat_zero_write(
        &repository,
        &["--json", "observe", "query"],
        &[0],
        "ObservabilityQuery-v1",
    );
    assert_eq!(released["event_count"], 1);
    assert_eq!(released["events"][0]["event_id"], "lock-deadline-event");
    assert_eq!(
        released["local_policy"]["lock_timeout_millis"],
        EventStore::supported_lock_timeout_millis()
    );
}

fn assert_public_lock_timeout(output: &Output, root: &Path, elapsed: Duration, timeout: Duration) {
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    assert_private_absent(output, root);
    let value = machine(&output.stderr);
    assert_eq!(value["schema_version"], "HarnessDiagnostic-v1");
    assert_eq!(
        value["diagnostic_id"],
        "successor_runtime_observability_unavailable"
    );
    assert_eq!(value["exit_class"], "actionable_finding");
    assert_eq!(
        value["cause"],
        "the bounded local event store lock deadline expired before a stable query could begin"
    );
    assert!(
        elapsed >= timeout.saturating_sub(Duration::from_millis(50)),
        "public timeout returned early: {elapsed:?}"
    );
    assert!(
        elapsed <= timeout + Duration::from_secs(2),
        "public timeout exceeded bound: {elapsed:?}"
    );
}

#[test]
fn cargo_supplied_binary_is_a_distinct_regular_executable() {
    let binary = Path::new(env!("CARGO_BIN_EXE_ultragoal"));
    let metadata = fs::symlink_metadata(binary).expect("Cargo binary metadata");
    assert!(metadata.is_file());
    assert!(!metadata.file_type().is_symlink());
    assert_ne!(metadata.permissions().mode() & 0o111, 0);
    assert_ne!(
        fs::canonicalize(binary).unwrap(),
        std::env::current_exe().unwrap()
    );
    let bytes = fs::read(binary).expect("read Cargo binary identity");
    assert!(bytes.len() > 1024);
    let digest = format!("sha256:{:x}", Sha256::digest(bytes));
    assert_eq!(digest.len(), "sha256:".len() + 64);
}
