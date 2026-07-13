#![cfg(target_vendor = "apple")]

use super::super::authority::{
    RepositoryFitApplyNonce, RepositoryFitAuthorityStore, RepositoryFitTrustedClock,
    after_effect_before_terminal_for_test, after_effect_start_before_apply_for_test,
    after_reservation_for_test, configure_effects_for_test, execute_prepared_apply,
    parse_recovery_intent, prepare_recovery_intent, recover_prepared_apply,
};
use super::super::catalog::CANONICAL_TEMPLATES;
use super::super::ledger::{
    FileRepositoryFitLedger, LedgerErrorId, RecoveryTargetRow, RecoveryTargetSpec,
    RepositoryFitLedgerState, ReservationDecision, ReservationRequest,
    before_atomic_publish_for_test, before_existing_open_for_test, before_lock_acquire_for_test,
    canonical_recovery_intent_bytes,
};
use super::super::root_permit::managed_ancestor_contract_for_ledger_test;
use super::super::{AdapterErrorId, plan_target, prepare_apply_request, verify_target};
use super::support::{git_status, snapshot};
use crate::context::{BuildRequest, LiveContext};
use crate::repository_fit::{
    FitAdapterError, FitReader, LocalRepository, PreparedFitApply, digest, inspect_target,
};
use std::collections::{BTreeMap, VecDeque};
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Barrier, Mutex, mpsc};
use std::time::{Duration, Instant};

const BASE: &str = "/private/tmp/hul-repository-fit-production-authority-085-fixtures";
static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

const AUTHORITY_SCENARIO_HELPER: &str = "repository_fit::product_adapter::tests::production_authority::subprocess_authority_scenario_helper";

struct ChildGuard(Option<Child>);

impl ChildGuard {
    fn new(child: Child) -> Self {
        Self(Some(child))
    }

    fn wait_with_output(mut self) -> Output {
        let child = self.0.take().expect("child is present");
        child.wait_with_output().expect("child output is available")
    }

    fn try_wait(&mut self) -> Option<std::process::ExitStatus> {
        self.0
            .as_mut()
            .expect("child is present")
            .try_wait()
            .expect("child status is available")
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Some(child) = self.0.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
struct ProofWorkerResultV1 {
    worker: String,
    lease_id: String,
    context_id: String,
    candidate_identity: BTreeMap<String, serde_json::Value>,
    base_state: BTreeMap<String, serde_json::Value>,
    final_state: BTreeMap<String, serde_json::Value>,
    touched_paths: Vec<String>,
    touched_semantics: Vec<String>,
    generated_outputs: Vec<String>,
    fixtures: Vec<String>,
    effects: Vec<serde_json::Value>,
    requirements: Vec<String>,
    dependency_nodes: Vec<String>,
    changes: Vec<BTreeMap<String, serde_json::Value>>,
    commands_and_tests: Vec<BTreeMap<String, serde_json::Value>>,
    artifacts: Vec<ProofArtifactRecord>,
    findings: Vec<BTreeMap<String, serde_json::Value>>,
    unresolved_dependencies: Vec<String>,
    requested_root_changes: Vec<serde_json::Value>,
    limitations: Vec<String>,
    no_claim_statement: String,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ProofArtifactRecord {
    path: String,
    sha256: String,
    byte_length: u64,
}

struct Fixture {
    container: PathBuf,
    root: PathBuf,
    store: TestStore,
}

impl Fixture {
    fn new(label: &str) -> Self {
        let serial = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let container = PathBuf::from(BASE).join(format!(
            "{}-{}-{serial}",
            label.replace(|character: char| !character.is_ascii_alphanumeric(), "-"),
            std::process::id()
        ));
        let root = container.join("repo");
        let store_root = container.join("authority");
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&store_root).unwrap();
        fs::set_permissions(&store_root, fs::Permissions::from_mode(0o700)).unwrap();
        git(&root, &["init", "--quiet"]);
        Self {
            container,
            root,
            store: TestStore {
                root: store_root,
                id: digest(format!("repository-fit-store:{label}:{serial}").as_bytes()),
            },
        }
    }

    fn context(&self) -> LiveContext {
        LiveContext::build(BuildRequest::new(&self.root)).unwrap()
    }

    fn prepared(&self, context: &LiveContext) -> PreparedFitApply {
        let plan = plan_target(context).unwrap();
        prepare_apply_request(
            context,
            &plan.to_machine_bytes().unwrap(),
            plan.plan_sha256(),
        )
        .unwrap()
    }

    fn write(&self, relative: &str, bytes: &[u8]) {
        let path = self.root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }

    fn write_template(&self, relative: &str) {
        let template = CANONICAL_TEMPLATES
            .iter()
            .find(|row| row.target_path == relative)
            .unwrap();
        self.write(relative, template.bytes);
        fs::set_permissions(
            self.root.join(relative),
            fs::Permissions::from_mode(template.unix_mode),
        )
        .unwrap();
    }

    fn install_all_direct(&self) {
        for row in CANONICAL_TEMPLATES {
            self.write(row.target_path, row.bytes);
            fs::set_permissions(
                self.root.join(row.target_path),
                fs::Permissions::from_mode(row.unix_mode),
            )
            .unwrap();
        }
    }

    fn store_names(&self) -> Vec<String> {
        let mut names = fs::read_dir(&self.store.root)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        names.sort();
        names
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        debug_assert!(self.container.starts_with(BASE));
        let _ = fs::remove_dir_all(&self.container);
    }
}

struct TestStore {
    root: PathBuf,
    id: String,
}

impl RepositoryFitAuthorityStore for TestStore {
    fn protected_root(&self) -> &Path {
        &self.root
    }

    fn store_id(&self) -> &str {
        &self.id
    }
}

struct TestClock {
    ticks: Mutex<VecDeque<u64>>,
}

impl TestClock {
    fn new(ticks: impl IntoIterator<Item = u64>) -> Self {
        Self {
            ticks: Mutex::new(ticks.into_iter().collect()),
        }
    }
}

impl RepositoryFitTrustedClock for TestClock {
    fn trusted_tick(&self) -> Result<u64, FitAdapterError> {
        self.ticks
            .lock()
            .unwrap()
            .pop_front()
            .ok_or_else(|| super::super::adapter_error(AdapterErrorId::ApplyPermitExpired))
    }
}

fn nonce(label: &str) -> RepositoryFitApplyNonce {
    RepositoryFitApplyNonce::new(digest(label.as_bytes()).into_bytes()).unwrap()
}

fn execute(
    fixture: &Fixture,
    context: &LiveContext,
    prepared: PreparedFitApply,
    label: &str,
) -> super::super::authority::RepositoryFitProductionOutcome {
    execute_with_ticks(fixture, context, prepared, label, [10, 11, 12])
}

fn execute_with_ticks(
    fixture: &Fixture,
    context: &LiveContext,
    prepared: PreparedFitApply,
    label: &str,
    ticks: impl IntoIterator<Item = u64>,
) -> super::super::authority::RepositoryFitProductionOutcome {
    let recovery_intent = prepare_recovery_intent(context, &prepared).unwrap();
    execute_prepared_apply(
        context,
        prepared,
        recovery_intent,
        &TestClock::new(ticks),
        &fixture.store,
        nonce(label),
    )
}

fn git(root: &Path, arguments: &[&str]) {
    let status = Command::new("/usr/bin/git")
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .args(arguments)
        .current_dir(root)
        .status()
        .unwrap();
    assert!(status.success());
}

fn fixed_digest(character: char) -> String {
    format!("sha256:{}", character.to_string().repeat(64))
}

fn reservation<'a>(character: char) -> ReservationRequest<'a> {
    let values = Box::leak(Box::new([
        fixed_digest(character),
        fixed_digest(char::from_u32(character as u32 + 1).unwrap()),
        fixed_digest(char::from_u32(character as u32 + 2).unwrap()),
        fixed_digest(char::from_u32(character as u32 + 3).unwrap()),
        fixed_digest(char::from_u32(character as u32 + 4).unwrap()),
        fixed_digest(char::from_u32(character as u32 + 5).unwrap()),
    ]));
    let recovery = Box::leak(Box::new(RecoveryTargetSpec {
        request_id: fixed_digest('f'),
        root_binding: fixed_digest('e'),
        ancestors: managed_ancestor_contract_for_ledger_test(),
        rows: vec![RecoveryTargetRow {
            path: "AGENTS.md".to_owned(),
            pre_sha256: None,
            pre_mode: None,
            post_sha256: fixed_digest('d'),
            post_mode: 0o644,
        }],
    }));
    values[5] = digest(&canonical_recovery_intent_bytes(recovery).unwrap());
    ReservationRequest {
        binding_sha256: &values[0],
        semantic_effect_id: &values[1],
        target_scope_id: &values[2],
        permit_id: &values[3],
        nonce_sha256: &values[4],
        recovery_intent_sha256: &values[5],
        issued_tick: 10,
        expires_tick: 20,
        recovery,
    }
}

fn authority_scenario_command(
    fixture: &Fixture,
    role: &str,
    nonce_label: &str,
    result_path: &Path,
    ready_path: &Path,
    release_path: &Path,
) -> Command {
    authority_scenario_command_at_root(
        fixture,
        &fixture.root,
        role,
        nonce_label,
        result_path,
        ready_path,
        release_path,
    )
}

fn authority_scenario_command_at_root(
    fixture: &Fixture,
    repository_root: &Path,
    role: &str,
    nonce_label: &str,
    result_path: &Path,
    ready_path: &Path,
    release_path: &Path,
) -> Command {
    let intent_path = recovery_intent_path(fixture, nonce_label);
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", AUTHORITY_SCENARIO_HELPER, "--nocapture"])
        .env("HUL_FIT_AUTHORITY_SCENARIO", role)
        .env("HUL_FIT_AUTHORITY_REPO", repository_root)
        .env("HUL_FIT_AUTHORITY_STORE", &fixture.store.root)
        .env("HUL_FIT_AUTHORITY_STORE_ID", &fixture.store.id)
        .env("HUL_FIT_AUTHORITY_NONCE_LABEL", nonce_label)
        .env("HUL_FIT_AUTHORITY_INTENT", intent_path)
        .env("HUL_FIT_AUTHORITY_RESULT", result_path)
        .env("HUL_FIT_AUTHORITY_READY", ready_path)
        .env("HUL_FIT_AUTHORITY_RELEASE", release_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
}

fn recovery_intent_path(fixture: &Fixture, nonce_label: &str) -> PathBuf {
    fixture.container.join(format!(
        "{}-recovery-intent.json",
        nonce_label.replace(|character: char| !character.is_ascii_alphanumeric(), "-")
    ))
}

fn wait_for_path(path: &Path) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while !path.exists() {
        assert!(Instant::now() < deadline, "timed out waiting for {path:?}");
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn result_value(path: &Path) -> serde_json::Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

fn run_authority_crash(
    fixture: &Fixture,
    role: &str,
    nonce_label: &str,
    exit_code: i32,
) -> PathBuf {
    let result = fixture.container.join(format!("{role}-result.json"));
    let ready = fixture.container.join(format!("{role}-ready"));
    let release = fixture.container.join(format!("{role}-release"));
    let output = authority_scenario_command(fixture, role, nonce_label, &result, &ready, &release)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(exit_code), "{output:?}");
    assert!(!result.exists());
    let intent = recovery_intent_path(fixture, nonce_label);
    let bytes = fs::read(&intent).expect("the recovery intent is durable before reservation");
    parse_recovery_intent(&bytes).expect("the persisted recovery intent is canonical");
    intent
}

fn run_authority_scenario(fixture: &Fixture, role: &str, nonce_label: &str) -> serde_json::Value {
    run_authority_scenario_at_root(fixture, &fixture.root, role, nonce_label)
}

fn run_authority_scenario_at_root(
    fixture: &Fixture,
    repository_root: &Path,
    role: &str,
    nonce_label: &str,
) -> serde_json::Value {
    let result = fixture.container.join(format!("{role}-result.json"));
    let ready = fixture.container.join(format!("{role}-ready"));
    let release = fixture.container.join(format!("{role}-release"));
    let output = authority_scenario_command_at_root(
        fixture,
        repository_root,
        role,
        nonce_label,
        &result,
        &ready,
        &release,
    )
    .output()
    .unwrap();
    assert!(output.status.success(), "{output:?}");
    result_value(&result)
}

fn observed_root_binding(root: &Path) -> String {
    let mut repository = LocalRepository::open(root).unwrap();
    repository.root_binding().unwrap()
}

fn create_existing_codex_ancestor(fixture: &Fixture) -> PathBuf {
    let path = fixture.root.join(".codex");
    fs::create_dir(&path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    path
}

fn replace_directory_preserving_children(path: &Path, quarantine: &Path) {
    let mode = fs::symlink_metadata(path).unwrap().mode() & 0o7777;
    fs::rename(path, quarantine).unwrap();
    fs::create_dir(path).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
    let mut entries = fs::read_dir(quarantine)
        .unwrap()
        .map(|entry| entry.unwrap())
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        fs::rename(entry.path(), path.join(entry.file_name())).unwrap();
    }
}

fn change_group_if_representable(path: &Path) -> bool {
    let current = fs::symlink_metadata(path).unwrap().gid();
    let count = unsafe { libc::getgroups(0, std::ptr::null_mut()) };
    if count <= 0 {
        return false;
    }
    let mut groups = vec![0 as libc::gid_t; count as usize];
    let observed = unsafe { libc::getgroups(count, groups.as_mut_ptr()) };
    if observed != count {
        return false;
    }
    let Some(alternate) = groups.into_iter().find(|group| *group != current) else {
        return false;
    };
    let encoded = std::ffi::CString::new(path.as_os_str().as_encoded_bytes()).unwrap();
    let changed = unsafe { libc::chown(encoded.as_ptr(), !0 as libc::uid_t, alternate) } == 0;
    if changed {
        assert_eq!(fs::symlink_metadata(path).unwrap().gid(), alternate);
    }
    changed
}

fn assert_expired_recovery_is_ambiguous(fixture: &Fixture, nonce_label: &str) {
    let recovered = run_authority_scenario(fixture, "recover-expired", nonce_label);
    assert_eq!(recovered["status"], "ambiguous");
    assert_eq!(recovered["adapter_error_id"], "apply_outcome_ambiguous");
    assert_eq!(recovered["ledger_state"], "ambiguous");
    assert_eq!(recovered["effect_started"], true);
    assert_eq!(recovered["effect"], "none");
    let terminal = fs::read_to_string(fixture.store.root.join("authority-ledger.json")).unwrap();
    assert_eq!(terminal.matches("\"state\":\"ambiguous\"").count(), 1);
    assert!(!terminal.contains("committed"));
}

#[test]
fn fresh_exact_apply_commits_and_repeat_is_idempotent() {
    let fixture = Fixture::new("fresh-commit-repeat");
    let context = fixture.context();
    let first = execute(
        &fixture,
        &context,
        fixture.prepared(&context),
        "fresh-commit",
    );
    assert_eq!(first.status(), "applied");
    assert_eq!(first.error_id(), None);
    assert!(first.effect_started());
    for row in CANONICAL_TEMPLATES {
        assert_eq!(
            fs::read(fixture.root.join(row.target_path)).unwrap(),
            row.bytes
        );
    }

    let repeated_context = fixture.context();
    let repeated = execute(
        &fixture,
        &repeated_context,
        fixture.prepared(&repeated_context),
        "fresh-idempotent",
    );
    assert_eq!(repeated.status(), "idempotent");
    assert_eq!(repeated.error_id(), None);
    assert!(!repeated.result_id().is_empty());
    assert_eq!(
        fixture.store_names(),
        ["authority-ledger.json", "authority.key", "authority.lock"]
    );
}

#[test]
fn partial_dirty_target_preserves_user_bytes_and_applies_only_plan() {
    let fixture = Fixture::new("partial-dirty");
    fixture.write("USER-NOTES.txt", b"keep these user bytes\n");
    fixture.write_template(CANONICAL_TEMPLATES[0].target_path);
    let before_user = fs::read(fixture.root.join("USER-NOTES.txt")).unwrap();
    let context = fixture.context();
    let outcome = execute(
        &fixture,
        &context,
        fixture.prepared(&context),
        "partial-dirty",
    );
    assert_eq!(outcome.status(), "applied");
    assert_eq!(
        fs::read(fixture.root.join("USER-NOTES.txt")).unwrap(),
        before_user
    );
    for row in CANONICAL_TEMPLATES {
        assert_eq!(
            fs::read(fixture.root.join(row.target_path)).unwrap(),
            row.bytes
        );
    }
}

#[test]
fn exact_effect_failure_rolls_back_prior_bytes_and_settles_terminal() {
    let fixture = Fixture::new("rollback");
    fixture.write_template(CANONICAL_TEMPLATES[0].target_path);
    let context = fixture.context();
    let prepared = fixture.prepared(&context);
    let before = snapshot(&fixture.root);
    configure_effects_for_test(|effects| effects.fail_on_calls([2]));
    let outcome = execute(&fixture, &context, prepared, "rollback");
    assert_eq!(outcome.status(), "rolled_back");
    assert_eq!(outcome.error_id(), Some(AdapterErrorId::ApplyRolledBack));
    assert!(outcome.effect_started());
    assert!(outcome.rollback_complete());
    assert_eq!(snapshot(&fixture.root), before);
    assert!(
        String::from_utf8(fs::read(fixture.store.root.join("authority-ledger.json")).unwrap())
            .unwrap()
            .contains("rolled_back")
    );
}

#[test]
fn unexpired_contender_refuses_and_expired_pre_effect_reservation_reconciles_without_target_write()
{
    let fixture = Fixture::new("interrupted");
    let context = fixture.context();
    let prepared = fixture.prepared(&context);
    let recovery_intent = prepare_recovery_intent(&context, &prepared).unwrap();
    let intent = recovery_intent.to_machine_bytes();
    let before = snapshot(&fixture.root);
    after_reservation_for_test(|| panic!("simulated process interruption"));
    let interrupted = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        execute_prepared_apply(
            &context,
            prepared,
            recovery_intent,
            &TestClock::new([10, 11, 12]),
            &fixture.store,
            nonce("interrupted-first"),
        )
    }));
    assert!(interrupted.is_err());
    assert_eq!(snapshot(&fixture.root), before);

    let active_context = fixture.context();
    let active = recover_prepared_apply(
        &active_context,
        &intent,
        &TestClock::new([22]),
        &fixture.store,
        nonce("interrupted-first"),
    );
    assert_eq!(active.status(), "refused");
    assert_eq!(active.error_id(), Some(AdapterErrorId::ApplyLeaseInvalid));
    assert!(!active.effect_started());
    assert_eq!(snapshot(&fixture.root), before);

    let retry_context = fixture.context();
    let retry = recover_prepared_apply(
        &retry_context,
        &intent,
        &TestClock::new([73]),
        &fixture.store,
        nonce("interrupted-first"),
    );
    assert_eq!(retry.status(), "interrupted");
    assert_eq!(
        retry.error_id(),
        Some(AdapterErrorId::ApplyOutcomeAmbiguous)
    );
    assert!(!retry.effect_started());
    assert_eq!(snapshot(&fixture.root), before);
    assert!(
        String::from_utf8(fs::read(fixture.store.root.join("authority-ledger.json")).unwrap())
            .unwrap()
            .contains("interrupted")
    );
}

#[test]
fn stale_target_and_invalid_clock_refuse_before_authority_store_write() {
    let stale = Fixture::new("stale-before-store");
    let context = stale.context();
    let prepared = stale.prepared(&context);
    let recovery_intent = prepare_recovery_intent(&context, &prepared).unwrap();
    stale.write(CANONICAL_TEMPLATES[0].target_path, b"attacker bytes\n");
    let before = snapshot(&stale.root);
    let refused = execute_prepared_apply(
        &context,
        prepared,
        recovery_intent,
        &TestClock::new([10, 11, 12]),
        &stale.store,
        nonce("stale-before-store"),
    );
    assert_eq!(refused.status(), "refused");
    assert_eq!(stale.store_names(), Vec::<String>::new());
    assert_eq!(snapshot(&stale.root), before);

    let expired = Fixture::new("clock-before-store");
    let context = expired.context();
    let prepared = expired.prepared(&context);
    let recovery_intent = prepare_recovery_intent(&context, &prepared).unwrap();
    let before = snapshot(&expired.root);
    let result = execute_prepared_apply(
        &context,
        prepared,
        recovery_intent,
        &TestClock::new([100, 99]),
        &expired.store,
        nonce("clock-before-store"),
    );
    assert_eq!(result.error_id(), Some(AdapterErrorId::ApplyPermitExpired));
    assert_eq!(expired.store_names(), Vec::<String>::new());
    assert_eq!(snapshot(&expired.root), before);
}

#[test]
fn managed_ancestor_replacement_invalidates_recovery_intent_before_store_open() {
    let fixture = Fixture::new("ancestor-intent-before-store");
    let ancestor = create_existing_codex_ancestor(&fixture);
    let context = fixture.context();
    let prepared = fixture.prepared(&context);
    let recovery_intent = prepare_recovery_intent(&context, &prepared).unwrap();
    replace_directory_preserving_children(
        &ancestor,
        &fixture.container.join("original-pre-reservation-codex"),
    );
    let before = snapshot(&fixture.root);

    let outcome = execute_prepared_apply(
        &context,
        prepared,
        recovery_intent,
        &TestClock::new([10, 11, 12]),
        &fixture.store,
        nonce("ancestor-intent-before-store"),
    );

    assert_eq!(outcome.status(), "refused");
    assert_eq!(outcome.error_id(), Some(AdapterErrorId::ApplyPermitInvalid));
    assert_eq!(fixture.store_names(), Vec::<String>::new());
    assert_eq!(snapshot(&fixture.root), before);
}

#[test]
fn post_reservation_target_mutation_is_rejected_without_authority_mutation() {
    let fixture = Fixture::new("post-reservation-target-race");
    let context = fixture.context();
    let prepared = fixture.prepared(&context);
    let target = fixture.root.join(CANONICAL_TEMPLATES[0].target_path);
    after_reservation_for_test(move || {
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(target, b"attacker raced after reservation\n").unwrap();
    });
    let outcome = execute(&fixture, &context, prepared, "post-reservation-race");
    assert_eq!(outcome.status(), "refused");
    assert_eq!(outcome.error_id(), Some(AdapterErrorId::StalePlan));
    assert!(!outcome.effect_started());
    assert_eq!(
        fs::read(fixture.root.join(CANONICAL_TEMPLATES[0].target_path)).unwrap(),
        b"attacker raced after reservation\n"
    );
    for row in &CANONICAL_TEMPLATES[1..] {
        assert!(!fixture.root.join(row.target_path).exists());
    }
    assert!(
        String::from_utf8(fs::read(fixture.store.root.join("authority-ledger.json")).unwrap())
            .unwrap()
            .contains("rejected")
    );
}

#[test]
fn symlink_hardlink_and_special_target_substitution_refuse_before_store() {
    for kind in ["symlink", "hardlink", "fifo"] {
        let fixture = Fixture::new(&format!("target-{kind}"));
        let context = fixture.context();
        let prepared = fixture.prepared(&context);
        let recovery_intent = prepare_recovery_intent(&context, &prepared).unwrap();
        let target = fixture.root.join(CANONICAL_TEMPLATES[0].target_path);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        match kind {
            "symlink" => symlink("../outside", &target).unwrap(),
            "hardlink" => {
                let source = fixture.root.join("hardlink-source");
                fs::write(&source, b"hardlink").unwrap();
                fs::hard_link(source, &target).unwrap();
            }
            "fifo" => {
                let path = std::ffi::CString::new(target.as_os_str().as_encoded_bytes()).unwrap();
                assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
            }
            _ => unreachable!(),
        }
        let outcome = execute_prepared_apply(
            &context,
            prepared,
            recovery_intent,
            &TestClock::new([10, 11, 12]),
            &fixture.store,
            nonce(kind),
        );
        assert_eq!(outcome.status(), "refused");
        assert_eq!(fixture.store_names(), Vec::<String>::new());
    }
}

#[test]
fn canonical_outcome_redacts_nonce_path_secret_and_backend_canaries() {
    let fixture = Fixture::new("redaction");
    let context = fixture.context();
    let canary = "zzq7V5-repository-fit-production-nonce-secret";
    let prepared = fixture.prepared(&context);
    let recovery_intent = prepare_recovery_intent(&context, &prepared).unwrap();
    let outcome = execute_prepared_apply(
        &context,
        prepared,
        recovery_intent,
        &TestClock::new([10, 11, 12]),
        &fixture.store,
        RepositoryFitApplyNonce::new(canary.repeat(2).into_bytes()).unwrap(),
    );
    assert_eq!(outcome.status(), "applied");
    let machine = String::from_utf8(outcome.to_machine_bytes().unwrap()).unwrap();
    assert!(!machine.contains(canary));
    assert!(!machine.contains(fixture.root.to_str().unwrap()));
    assert!(!machine.contains(fixture.store.root.to_str().unwrap()));
    assert!(!machine.contains("raw-backend-output-canary"));
    assert!(machine.len() <= 16 * 1024);
}

#[test]
fn inspect_plan_verify_and_verify_as_apply_are_recursively_zero_write() {
    let fixture = Fixture::new("zero-write-reads");
    let context = fixture.context();
    let target_before = snapshot(&fixture.root);
    let status_before = git_status(&fixture.root);
    let _ = inspect_target(&context).unwrap();
    let _ = plan_target(&context).unwrap();
    let _verify = verify_target(&context).unwrap();
    assert_eq!(snapshot(&fixture.root), target_before);
    assert_eq!(git_status(&fixture.root), status_before);
    assert_eq!(fixture.store_names(), Vec::<String>::new());
}

#[test]
fn forged_desired_target_after_interruption_cannot_become_terminal_success() {
    let fixture = Fixture::new("forged-success");
    let context = fixture.context();
    let prepared = fixture.prepared(&context);
    let intent = prepare_recovery_intent(&context, &prepared).unwrap();
    let intent_bytes = intent.to_machine_bytes();
    after_reservation_for_test(|| panic!("interrupt before effect"));
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        execute_prepared_apply(
            &context,
            prepared,
            intent,
            &TestClock::new([10, 11, 12]),
            &fixture.store,
            nonce("forged-success-nonce"),
        )
    }));
    fixture.install_all_direct();
    let forged_context = fixture.context();
    let forged = recover_prepared_apply(
        &forged_context,
        &intent_bytes,
        &TestClock::new([73]),
        &fixture.store,
        nonce("forged-success-nonce"),
    );
    assert_eq!(forged.status(), "interrupted");
    assert_eq!(
        forged.error_id(),
        Some(AdapterErrorId::ApplyOutcomeAmbiguous)
    );
    let ledger =
        String::from_utf8(fs::read(fixture.store.root.join("authority-ledger.json")).unwrap())
            .unwrap();
    assert!(!ledger.contains("committed"));
}

#[test]
fn ledger_rejects_key_lock_state_root_and_special_file_substitution() {
    for attack in [
        "key",
        "lock",
        "state",
        "truncated",
        "unknown-row",
        "hardlink",
        "root-mode",
    ] {
        let fixture = Fixture::new(&format!("ledger-{attack}"));
        let ledger = FileRepositoryFitLedger::open_or_initialize(
            &fixture.store.root,
            fixture.store.store_id(),
        )
        .unwrap();
        let token = match ledger.reserve(reservation('1')).unwrap() {
            ReservationDecision::Acquired(token) => token,
            ReservationDecision::Existing(_) => unreachable!(),
        };
        ledger
            .terminal(
                token,
                RepositoryFitLedgerState::Rejected,
                &fixed_digest('9'),
                Some(AdapterErrorId::ApplyPermitInvalid),
                11,
            )
            .unwrap();
        let key = fixture.store.root.join("authority.key");
        let lock = fixture.store.root.join("authority.lock");
        let state = fixture.store.root.join("authority-ledger.json");
        match attack {
            "key" => {
                let held = fixture.store.root.join("authority.key.held");
                fs::rename(&key, held).unwrap();
                fs::write(&key, [7u8; 32]).unwrap();
                fs::set_permissions(&key, fs::Permissions::from_mode(0o600)).unwrap();
            }
            "lock" => {
                let held = fixture.store.root.join("authority.lock.held");
                fs::rename(&lock, held).unwrap();
                fs::write(&lock, b"repository-fit-authority-lock-v2\n").unwrap();
                fs::set_permissions(&lock, fs::Permissions::from_mode(0o600)).unwrap();
            }
            "state" => fs::write(&state, b"{}").unwrap(),
            "truncated" => {
                let bytes = fs::read(&state).unwrap();
                fs::write(&state, &bytes[..bytes.len() / 2]).unwrap();
            }
            "unknown-row" => {
                let bytes = String::from_utf8(fs::read(&state).unwrap()).unwrap();
                fs::write(&state, bytes.replacen('{', "{\"unknown\":true,", 1)).unwrap();
            }
            "hardlink" => {
                fs::hard_link(
                    &state,
                    fixture.store.root.join("authority-ledger-copy.json"),
                )
                .unwrap();
            }
            "root-mode" => {
                fs::set_permissions(&fixture.store.root, fs::Permissions::from_mode(0o755))
                    .unwrap();
            }
            _ => unreachable!(),
        }
        let result = ledger.reserve(reservation('a'));
        assert!(
            matches!(result, Err(ref error) if matches!(error.id(), LedgerErrorId::Tampered | LedgerErrorId::InvalidStore | LedgerErrorId::Io)),
            "attack {attack} did not fail closed"
        );
    }

    for kind in ["symlink-root", "fifo-key", "unknown-without-lock"] {
        let fixture = Fixture::new(kind);
        let requested = if kind == "symlink-root" {
            let real = fixture.container.join("real-authority");
            fs::create_dir(&real).unwrap();
            fs::set_permissions(&real, fs::Permissions::from_mode(0o700)).unwrap();
            let alias = fixture.container.join("authority-alias");
            symlink(&real, &alias).unwrap();
            alias
        } else if kind == "fifo-key" {
            let key = fixture.store.root.join("authority.key");
            let path = std::ffi::CString::new(key.as_os_str().as_encoded_bytes()).unwrap();
            assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
            fixture.store.root.clone()
        } else {
            fs::write(fixture.store.root.join("unrecognized-row"), b"attacker\n").unwrap();
            fixture.store.root.clone()
        };
        let before = fs::read_dir(&requested)
            .map(|rows| rows.count())
            .unwrap_or_default();
        let result =
            FileRepositoryFitLedger::open_or_initialize(&requested, fixture.store.store_id());
        assert!(result.is_err());
        let after = fs::read_dir(&requested)
            .map(|rows| rows.count())
            .unwrap_or_default();
        assert_eq!(after, before);
        assert!(!requested.join("authority.lock").exists());
    }
}

#[test]
fn same_session_stale_head_replay_and_terminal_substitution_fail_closed() {
    let fixture = Fixture::new("stale-head");
    let ledger =
        FileRepositoryFitLedger::open_or_initialize(&fixture.store.root, fixture.store.store_id())
            .unwrap();
    let token = match ledger.reserve(reservation('1')).unwrap() {
        ReservationDecision::Acquired(token) => token,
        ReservationDecision::Existing(_) => unreachable!(),
    };
    let reserved = ledger.snapshot_for_test().unwrap();
    ledger
        .terminal(
            token,
            RepositoryFitLedgerState::Rejected,
            &fixed_digest('9'),
            Some(AdapterErrorId::ApplyPermitInvalid),
            11,
        )
        .unwrap();
    fs::write(fixture.store.root.join("authority-ledger.json"), reserved).unwrap();
    assert!(matches!(
        ledger.reserve(reservation('a')),
        Err(ref error) if error.id() == LedgerErrorId::Tampered
    ));

    let other = Fixture::new("terminal-substitution");
    let other_ledger =
        FileRepositoryFitLedger::open_or_initialize(&other.store.root, other.store.store_id())
            .unwrap();
    let token = match other_ledger.reserve(reservation('1')).unwrap() {
        ReservationDecision::Acquired(token) => token,
        ReservationDecision::Existing(_) => unreachable!(),
    };
    other_ledger
        .terminal(
            token,
            RepositoryFitLedgerState::Rejected,
            &fixed_digest('9'),
            Some(AdapterErrorId::ApplyPermitInvalid),
            11,
        )
        .unwrap();
    let state = other.store.root.join("authority-ledger.json");
    let bytes = String::from_utf8(fs::read(&state).unwrap()).unwrap();
    fs::write(&state, bytes.replacen("rejected", "committed", 1)).unwrap();
    assert!(matches!(
        other_ledger.reserve(reservation('a')),
        Err(ref error) if error.id() == LedgerErrorId::Tampered
    ));
}

#[test]
fn two_process_execute_through_terminal_allows_only_the_reserved_owner_to_mutate() {
    let fixture = Fixture::new("full-process-owner-race");
    let before = snapshot(&fixture.root);
    let ready = fixture.container.join("winner-reserved");
    let release = fixture.container.join("release-winner");
    let winner_result = fixture.container.join("winner-result.json");
    let contender_result = fixture.container.join("contender-result.json");

    let winner = authority_scenario_command(
        &fixture,
        "winner",
        "full-process-owner",
        &winner_result,
        &ready,
        &release,
    )
    .spawn()
    .unwrap();
    let winner = ChildGuard::new(winner);
    wait_for_path(&ready);

    let contender_output = authority_scenario_command(
        &fixture,
        "contender",
        "full-process-owner",
        &contender_result,
        &ready,
        &release,
    )
    .output()
    .unwrap();
    let before_release = snapshot(&fixture.root);
    let reserved_ledger = fs::read_to_string(fixture.store.root.join("authority-ledger.json"))
        .expect("the winner reservation is durable");
    fs::write(&release, b"release\n").unwrap();
    let winner_output = winner.wait_with_output();

    assert!(contender_output.status.success(), "{contender_output:?}");
    assert!(winner_output.status.success(), "{winner_output:?}");
    assert_eq!(before_release, before, "the contender changed the target");
    assert!(reserved_ledger.contains("reserved"));
    assert!(!reserved_ledger.contains("effect_started"));
    assert!(!reserved_ledger.contains("committed"));

    let contender = result_value(&contender_result);
    assert_eq!(contender["status"], "refused");
    assert_eq!(contender["adapter_error_id"], "apply_lease_invalid");
    assert_eq!(contender["ledger_state"], "reserved");
    assert_eq!(contender["effect_started"], false);
    assert_eq!(contender["effect"], "none");

    let winner = result_value(&winner_result);
    assert_eq!(winner["status"], "applied");
    assert_eq!(winner["ledger_state"], "committed");
    assert_eq!(winner["effect_started"], true);
    assert_eq!(winner["effect"], "workspace_write");
    assert_eq!(
        winner["apply_outcome"]["mutation_count"],
        CANONICAL_TEMPLATES.len()
    );
    for row in CANONICAL_TEMPLATES {
        assert_eq!(
            fs::read(fixture.root.join(row.target_path)).unwrap(),
            row.bytes
        );
    }
    let terminal = fs::read_to_string(fixture.store.root.join("authority-ledger.json")).unwrap();
    assert_eq!(terminal.matches("\"state\":\"reserved\"").count(), 1);
    assert_eq!(terminal.matches("\"state\":\"effect_started\"").count(), 1);
    assert_eq!(terminal.matches("\"state\":\"committed\"").count(), 1);
    assert!(!terminal.contains("interrupted"));
    assert!(!terminal.contains("ambiguous"));
}

#[test]
fn live_effect_owner_holds_process_lock_through_mutation_and_terminal() {
    let fixture = Fixture::new("effect-owner-lock-race");
    let owner_ready = fixture.container.join("effect-owner-ready");
    let owner_release = fixture.container.join("effect-owner-release");
    let owner_result = fixture.container.join("effect-owner-result.json");
    let recovery_ready = fixture.container.join("effect-recovery-ready");
    let recovery_release = fixture.container.join("effect-recovery-unused-release");
    let recovery_result = fixture.container.join("effect-recovery-result.json");

    let owner = authority_scenario_command(
        &fixture,
        "winner-effect",
        "effect-owner-lock",
        &owner_result,
        &owner_ready,
        &owner_release,
    )
    .spawn()
    .unwrap();
    let owner = ChildGuard::new(owner);
    wait_for_path(&owner_ready);

    let recovery = authority_scenario_command(
        &fixture,
        "recover-expired-signaled",
        "effect-owner-lock",
        &recovery_result,
        &recovery_ready,
        &recovery_release,
    )
    .spawn()
    .unwrap();
    let mut recovery = ChildGuard::new(recovery);
    wait_for_path(&recovery_ready);
    let blocked_until = Instant::now() + Duration::from_millis(100);
    while Instant::now() < blocked_until {
        assert!(
            recovery.try_wait().is_none(),
            "recovery escaped while the live effect owner held the exact lock"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(!recovery_result.exists());

    fs::write(&owner_release, b"release\n").unwrap();
    let owner_output = owner.wait_with_output();
    let recovery_output = recovery.wait_with_output();
    assert!(owner_output.status.success(), "{owner_output:?}");
    assert!(recovery_output.status.success(), "{recovery_output:?}");

    let owner = result_value(&owner_result);
    assert_eq!(owner["status"], "applied");
    assert_eq!(owner["ledger_state"], "committed");
    let recovery = result_value(&recovery_result);
    assert_eq!(recovery["status"], "refused");
    assert_eq!(recovery["adapter_error_id"], "apply_permit_replayed");
    assert_eq!(recovery["ledger_state"], "committed");
    assert_eq!(recovery["effect"], "none");

    let terminal = fs::read_to_string(fixture.store.root.join("authority-ledger.json")).unwrap();
    assert_eq!(terminal.matches("\"state\":\"reserved\"").count(), 1);
    assert_eq!(terminal.matches("\"state\":\"effect_started\"").count(), 1);
    assert_eq!(terminal.matches("\"state\":\"committed\"").count(), 1);
    assert!(!terminal.contains("interrupted"));
    assert!(!terminal.contains("ambiguous"));
}

#[test]
fn two_processes_racing_one_nonce_and_semantic_effect_have_one_ledger_winner() {
    let fixture = Fixture::new("process-race");
    let _ledger =
        FileRepositoryFitLedger::open_or_initialize(&fixture.store.root, fixture.store.store_id())
            .unwrap();
    let barrier = fixture.container.join("race-start");
    let reached = Arc::new(Barrier::new(3));
    let mut children = Vec::new();
    for index in 0..2 {
        let root = fixture.store.root.clone();
        let id = fixture.store.id.clone();
        let barrier = barrier.clone();
        let reached = Arc::clone(&reached);
        children.push(std::thread::spawn(move || {
            reached.wait();
            if index == 0 {
                fs::write(&barrier, b"go").unwrap();
            } else {
                while !barrier.exists() {
                    std::thread::yield_now();
                }
            }
            Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "repository_fit::product_adapter::tests::production_authority::subprocess_reservation_helper",
                    "--nocapture",
                ])
                .env("HUL_FIT_AUTHORITY_SUBPROCESS", "1")
                .env("HUL_FIT_AUTHORITY_STORE", root)
                .env("HUL_FIT_AUTHORITY_STORE_ID", id)
                .output()
                .unwrap()
        }));
    }
    reached.wait();
    let outputs = children
        .into_iter()
        .map(|child| child.join().unwrap())
        .collect::<Vec<_>>();
    for output in &outputs {
        assert!(output.status.success(), "{output:?}");
    }
    let joined = outputs
        .iter()
        .map(|output| String::from_utf8_lossy(&output.stdout))
        .collect::<String>();
    assert_eq!(joined.matches("RESERVATION-WINNER").count(), 1, "{joined}");
    assert_eq!(
        joined.matches("RESERVATION-EXISTING").count(),
        1,
        "{joined}"
    );
}

#[test]
fn opener_waits_on_the_exact_lock_while_a_legitimate_atomic_publication_is_in_flight() {
    let fixture = Fixture::new("atomic-publication-open-race");
    let writer_ledger =
        FileRepositoryFitLedger::open_or_initialize(&fixture.store.root, fixture.store.store_id())
            .unwrap();
    let (publish_reached_tx, publish_reached_rx) = mpsc::sync_channel(0);
    let (release_publish_tx, release_publish_rx) = mpsc::sync_channel(0);
    let writer = std::thread::spawn(move || {
        before_atomic_publish_for_test(move || {
            publish_reached_tx.send(()).unwrap();
            release_publish_rx.recv().unwrap();
        });
        match writer_ledger.reserve(reservation('1')).unwrap() {
            ReservationDecision::Acquired(_) => "winner",
            ReservationDecision::Existing(_) => "existing",
        }
    });
    publish_reached_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("writer must pause with its temporary state file fully written under the lock");

    let opener_root = fixture.store.root.clone();
    let opener_store_id = fixture.store.id.clone();
    let (opener_reached_tx, opener_reached_rx) = mpsc::sync_channel(0);
    let opener = std::thread::spawn(move || {
        before_lock_acquire_for_test(move || opener_reached_tx.send(()).unwrap());
        let ledger = FileRepositoryFitLedger::open_or_initialize(&opener_root, &opener_store_id)
            .expect("an in-flight legitimate publication is not store tampering");
        match ledger.reserve(reservation('1')).unwrap() {
            ReservationDecision::Acquired(_) => "winner",
            ReservationDecision::Existing(existing) => {
                assert_eq!(existing.state(), RepositoryFitLedgerState::Reserved);
                "existing"
            }
        }
    });
    opener_reached_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("opener must bind the exact lock without enumerating transient state");
    release_publish_tx.send(()).unwrap();

    assert_eq!(writer.join().unwrap(), "winner");
    assert_eq!(opener.join().unwrap(), "existing");
}

#[test]
fn missing_recovery_ledger_refuses_without_initializing_authority_or_writing_target() {
    let fixture = Fixture::new("missing-recovery-ledger");
    let context = fixture.context();
    let intent = prepare_recovery_intent(&context, &fixture.prepared(&context))
        .unwrap()
        .to_machine_bytes();
    let before_target = snapshot(&fixture.root);
    let before_status = git_status(&fixture.root);
    let before_store = fixture.store_names();
    assert!(before_store.is_empty());

    let outcome = recover_prepared_apply(
        &context,
        &intent,
        &TestClock::new([73]),
        &fixture.store,
        nonce("missing-recovery-ledger-owner"),
    );

    assert_eq!(outcome.status(), "refused");
    assert_eq!(
        outcome.error_id(),
        Some(AdapterErrorId::ApplyPermitReplayed)
    );
    assert_eq!(fixture.store_names(), before_store);
    assert_eq!(snapshot(&fixture.root), before_target);
    assert_eq!(git_status(&fixture.root), before_status);
}

#[test]
fn recovery_existing_only_open_rejects_whole_authority_substitution_without_reinitializing() {
    let fixture = Fixture::new("recovery-existing-only-substitution");
    let intent_path =
        run_authority_crash(&fixture, "crash-before", "recovery-existing-only-owner", 86);
    let intent = fs::read(intent_path).unwrap();
    let before_target = snapshot(&fixture.root);
    let before_status = git_status(&fixture.root);
    let repository_root = fixture.root.clone();
    let store_root = fixture.store.root.clone();
    let store_id = fixture.store.id.clone();
    let (open_reached_tx, open_reached_rx) = mpsc::sync_channel(0);
    let (release_open_tx, release_open_rx) = mpsc::sync_channel(0);
    let recovery = std::thread::spawn(move || {
        before_existing_open_for_test(move || {
            open_reached_tx.send(()).unwrap();
            release_open_rx.recv().unwrap();
        });
        let context = LiveContext::build(BuildRequest::new(&repository_root)).unwrap();
        let store = TestStore {
            root: store_root,
            id: store_id,
        };
        let outcome = recover_prepared_apply(
            &context,
            &intent,
            &TestClock::new([73]),
            &store,
            nonce("recovery-existing-only-owner"),
        );
        (outcome.status().to_owned(), outcome.error_id())
    });
    open_reached_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("recovery must pause after binding the store root and observing the named lock");

    let quarantine = fixture.container.join("removed-authority");
    fs::create_dir(&quarantine).unwrap();
    let original_names = fixture.store_names();
    assert_eq!(
        original_names,
        ["authority-ledger.json", "authority.key", "authority.lock"]
    );
    for name in &original_names {
        fs::rename(fixture.store.root.join(name), quarantine.join(name)).unwrap();
    }
    let quarantined = original_names
        .iter()
        .map(|name| (name.clone(), fs::read(quarantine.join(name)).unwrap()))
        .collect::<BTreeMap<_, _>>();
    assert!(fixture.store_names().is_empty());
    release_open_tx.send(()).unwrap();

    let (status, error_id) = recovery.join().unwrap();
    assert_eq!(status, "refused");
    assert_eq!(error_id, Some(AdapterErrorId::ApplyOutcomeInvalid));
    assert!(fixture.store_names().is_empty());
    for (name, bytes) in quarantined {
        assert_eq!(fs::read(quarantine.join(name)).unwrap(), bytes);
    }
    assert_eq!(snapshot(&fixture.root), before_target);
    assert_eq!(git_status(&fixture.root), before_status);
}

#[test]
fn abrupt_process_exit_before_effect_recovers_only_after_expiry_without_target_write() {
    let fixture = Fixture::new("process-crash-before-effect");
    let before = snapshot(&fixture.root);
    let _intent = run_authority_crash(&fixture, "crash-before", "crash-before-owner", 86);

    assert_eq!(snapshot(&fixture.root), before);
    let reserved = fs::read_to_string(fixture.store.root.join("authority-ledger.json")).unwrap();
    assert_eq!(reserved.matches("\"state\":\"reserved\"").count(), 1);
    assert!(!reserved.contains("effect_started"));
    assert!(!reserved.contains("committed"));

    let active = run_authority_scenario(&fixture, "recover-active", "crash-before-owner");
    assert_eq!(active["status"], "refused");
    assert_eq!(active["adapter_error_id"], "apply_lease_invalid");
    assert_eq!(active["ledger_state"], "reserved");
    assert_eq!(active["effect_started"], false);
    assert_eq!(snapshot(&fixture.root), before);

    let ordinary = run_authority_scenario(&fixture, "expired-execute", "crash-before-owner");
    assert_eq!(ordinary["status"], "refused");
    assert_eq!(ordinary["adapter_error_id"], "apply_lease_invalid");
    assert_eq!(ordinary["ledger_state"], "reserved");
    assert_eq!(ordinary["effect_started"], false);
    assert_eq!(snapshot(&fixture.root), before);

    let expired = run_authority_scenario(&fixture, "recover-expired", "crash-before-owner");
    assert_eq!(expired["status"], "interrupted");
    assert_eq!(expired["adapter_error_id"], "apply_outcome_ambiguous");
    assert_eq!(expired["ledger_state"], "interrupted");
    assert_eq!(expired["effect_started"], false);
    assert_eq!(expired["effect"], "none");
    assert_eq!(snapshot(&fixture.root), before);

    let terminal = fs::read_to_string(fixture.store.root.join("authority-ledger.json")).unwrap();
    assert_eq!(terminal.matches("\"state\":\"reserved\"").count(), 1);
    assert_eq!(terminal.matches("\"state\":\"interrupted\"").count(), 1);
    assert!(!terminal.contains("effect_started"));
    assert!(!terminal.contains("committed"));
}

#[test]
fn abrupt_process_exit_after_effect_with_same_root_binding_recovers_exact_postimage_as_committed() {
    let fixture = Fixture::new("process-crash-after-effect");
    let prepared_root_binding = observed_root_binding(&fixture.root);
    let _intent = run_authority_crash(&fixture, "crash-after", "crash-after-owner", 88);
    assert_eq!(observed_root_binding(&fixture.root), prepared_root_binding);

    for row in CANONICAL_TEMPLATES {
        assert_eq!(
            fs::read(fixture.root.join(row.target_path)).unwrap(),
            row.bytes
        );
    }
    let started = fs::read_to_string(fixture.store.root.join("authority-ledger.json")).unwrap();
    assert_eq!(started.matches("\"state\":\"reserved\"").count(), 1);
    assert_eq!(started.matches("\"state\":\"effect_started\"").count(), 1);
    assert!(!started.contains("committed"));

    let recovered = run_authority_scenario(&fixture, "recover-expired", "crash-after-owner");
    assert_eq!(recovered["status"], "recovered");
    assert_eq!(recovered["adapter_error_id"], serde_json::Value::Null);
    assert_eq!(recovered["effect_started"], true);
    assert_eq!(recovered["effect"], "none");
    assert_eq!(recovered["ledger_state"], "committed");

    let terminal = fs::read_to_string(fixture.store.root.join("authority-ledger.json")).unwrap();
    assert_eq!(terminal.matches("\"state\":\"reserved\"").count(), 1);
    assert_eq!(terminal.matches("\"state\":\"effect_started\"").count(), 1);
    assert_eq!(terminal.matches("\"state\":\"committed\"").count(), 1);
    assert!(!terminal.contains("interrupted"));
    assert!(!terminal.contains("ambiguous"));
}

#[test]
fn whole_root_rename_with_exact_leaf_postimage_cannot_recover_as_committed() {
    let fixture = Fixture::new("process-crash-after-effect-root-rename");
    let prepared_root_binding = observed_root_binding(&fixture.root);
    let original_root = fs::symlink_metadata(&fixture.root).unwrap();
    let _intent = run_authority_crash(&fixture, "crash-after", "crash-after-root-rename-owner", 88);
    let exact_postimage = snapshot(&fixture.root);
    let exact_post_status = git_status(&fixture.root);

    let relocated_root = fixture.container.join("relocated-repo");
    fs::rename(&fixture.root, &relocated_root).unwrap();
    let relocated_metadata = fs::symlink_metadata(&relocated_root).unwrap();
    assert_eq!(relocated_metadata.dev(), original_root.dev());
    assert_eq!(relocated_metadata.ino(), original_root.ino());
    assert_eq!(snapshot(&relocated_root), exact_postimage);
    assert_eq!(git_status(&relocated_root), exact_post_status);
    let relocated_root_binding = observed_root_binding(&relocated_root);
    assert_ne!(relocated_root_binding, prepared_root_binding);

    let before_recovery = snapshot(&relocated_root);
    let before_recovery_status = git_status(&relocated_root);
    let recovered = run_authority_scenario_at_root(
        &fixture,
        &relocated_root,
        "recover-expired",
        "crash-after-root-rename-owner",
    );
    assert_eq!(recovered["status"], "ambiguous");
    assert_eq!(recovered["adapter_error_id"], "apply_outcome_ambiguous");
    assert_eq!(recovered["ledger_state"], "ambiguous");
    assert_eq!(recovered["effect_started"], true);
    assert_eq!(recovered["effect"], "none");
    assert_eq!(snapshot(&relocated_root), before_recovery);
    assert_eq!(git_status(&relocated_root), before_recovery_status);
    assert!(!fixture.root.exists());

    let terminal = fs::read_to_string(fixture.store.root.join("authority-ledger.json")).unwrap();
    assert_eq!(terminal.matches("\"state\":\"reserved\"").count(), 1);
    assert_eq!(terminal.matches("\"state\":\"effect_started\"").count(), 1);
    assert_eq!(terminal.matches("\"state\":\"ambiguous\"").count(), 1);
    assert!(!terminal.contains("committed"));
}

#[test]
fn abrupt_process_exit_with_exact_existing_ancestor_postimage_recovers_as_committed() {
    let fixture = Fixture::new("process-crash-existing-ancestor-exact");
    let ancestor = create_existing_codex_ancestor(&fixture);
    let identity = fs::symlink_metadata(&ancestor).unwrap().ino();
    let _intent = run_authority_crash(
        &fixture,
        "crash-after",
        "crash-existing-ancestor-exact-owner",
        88,
    );
    assert_eq!(fs::symlink_metadata(&ancestor).unwrap().ino(), identity);

    let recovered = run_authority_scenario(
        &fixture,
        "recover-expired",
        "crash-existing-ancestor-exact-owner",
    );
    assert_eq!(recovered["status"], "recovered");
    assert_eq!(recovered["adapter_error_id"], serde_json::Value::Null);
    assert_eq!(recovered["ledger_state"], "committed");
    assert_eq!(recovered["effect_started"], true);
}

#[test]
fn existing_ancestor_replacement_with_identical_leaf_postimage_cannot_recover_as_committed() {
    let fixture = Fixture::new("process-crash-existing-ancestor-replaced");
    let ancestor = create_existing_codex_ancestor(&fixture);
    let original_identity = fs::symlink_metadata(&ancestor).unwrap().ino();
    let _intent = run_authority_crash(
        &fixture,
        "crash-after",
        "crash-existing-ancestor-replaced-owner",
        88,
    );
    let expected_leaves = CANONICAL_TEMPLATES
        .iter()
        .filter(|row| row.target_path.starts_with(".codex/"))
        .map(|row| {
            (
                row.target_path,
                fs::read(fixture.root.join(row.target_path)).unwrap(),
                fs::symlink_metadata(fixture.root.join(row.target_path))
                    .unwrap()
                    .mode()
                    & 0o7777,
            )
        })
        .collect::<Vec<_>>();
    let quarantine = fixture.container.join("original-codex-ancestor");
    replace_directory_preserving_children(&ancestor, &quarantine);
    assert_ne!(
        fs::symlink_metadata(&ancestor).unwrap().ino(),
        original_identity
    );
    for (path, bytes, mode) in expected_leaves {
        assert_eq!(fs::read(fixture.root.join(path)).unwrap(), bytes);
        assert_eq!(
            fs::symlink_metadata(fixture.root.join(path))
                .unwrap()
                .mode()
                & 0o7777,
            mode
        );
    }

    assert_expired_recovery_is_ambiguous(&fixture, "crash-existing-ancestor-replaced-owner");
}

#[test]
fn existing_ancestor_mode_drift_cannot_recover_as_committed() {
    let fixture = Fixture::new("process-crash-existing-ancestor-mode-drift");
    let ancestor = create_existing_codex_ancestor(&fixture);
    let _intent = run_authority_crash(
        &fixture,
        "crash-after",
        "crash-existing-ancestor-mode-owner",
        88,
    );
    fs::set_permissions(&ancestor, fs::Permissions::from_mode(0o700)).unwrap();
    assert_expired_recovery_is_ambiguous(&fixture, "crash-existing-ancestor-mode-owner");
}

#[test]
fn existing_ancestor_group_drift_cannot_recover_as_committed_when_representable() {
    let fixture = Fixture::new("process-crash-existing-ancestor-group-drift");
    let ancestor = create_existing_codex_ancestor(&fixture);
    let _intent = run_authority_crash(
        &fixture,
        "crash-after",
        "crash-existing-ancestor-group-owner",
        88,
    );
    if change_group_if_representable(&ancestor) {
        assert_expired_recovery_is_ambiguous(&fixture, "crash-existing-ancestor-group-owner");
    } else {
        eprintln!("managed ancestor group drift is not representable for this host user");
    }
}

#[test]
fn existing_ancestor_symlink_containment_alias_cannot_recover_as_committed() {
    let fixture = Fixture::new("process-crash-existing-ancestor-alias");
    let ancestor = create_existing_codex_ancestor(&fixture);
    let _intent = run_authority_crash(
        &fixture,
        "crash-after",
        "crash-existing-ancestor-alias-owner",
        88,
    );
    let quarantine = fixture.container.join("aliased-codex-ancestor");
    fs::rename(&ancestor, &quarantine).unwrap();
    symlink(&quarantine, &ancestor).unwrap();
    for row in CANONICAL_TEMPLATES
        .iter()
        .filter(|row| row.target_path.starts_with(".codex/"))
    {
        assert_eq!(
            fs::read(fixture.root.join(row.target_path)).unwrap(),
            row.bytes
        );
    }

    assert_expired_recovery_is_ambiguous(&fixture, "crash-existing-ancestor-alias-owner");
}

#[test]
fn abrupt_process_exit_after_effect_with_target_substitution_recovers_as_ambiguous() {
    let fixture = Fixture::new("process-crash-after-effect-substitution");
    let _intent = run_authority_crash(
        &fixture,
        "crash-after",
        "crash-after-substitution-owner",
        88,
    );
    let attacked = fixture.root.join(CANONICAL_TEMPLATES[0].target_path);
    fs::write(&attacked, b"attacker substituted after the owner exited\n").unwrap();

    let recovered = run_authority_scenario(
        &fixture,
        "recover-expired",
        "crash-after-substitution-owner",
    );
    assert_eq!(recovered["status"], "ambiguous");
    assert_eq!(recovered["adapter_error_id"], "apply_outcome_ambiguous");
    assert_eq!(recovered["effect_started"], true);
    assert_eq!(
        fs::read(&attacked).unwrap(),
        b"attacker substituted after the owner exited\n"
    );
    assert_eq!(recovered["effect"], "none");
    assert_eq!(recovered["ledger_state"], "ambiguous");

    let terminal = fs::read_to_string(fixture.store.root.join("authority-ledger.json")).unwrap();
    assert_eq!(terminal.matches("\"state\":\"reserved\"").count(), 1);
    assert_eq!(terminal.matches("\"state\":\"effect_started\"").count(), 1);
    assert_eq!(terminal.matches("\"state\":\"ambiguous\"").count(), 1);
    assert!(!terminal.contains("committed"));
}

#[test]
fn abrupt_process_exit_after_effect_start_with_partial_postimage_recovers_as_ambiguous() {
    let fixture = Fixture::new("process-crash-partial-effect");
    let before = snapshot(&fixture.root);
    let _intent = run_authority_crash(&fixture, "crash-after-start", "crash-partial-owner", 89);
    assert_eq!(snapshot(&fixture.root), before);
    fixture.write_template(CANONICAL_TEMPLATES[0].target_path);
    let partial = snapshot(&fixture.root);

    let recovered = run_authority_scenario(&fixture, "recover-expired", "crash-partial-owner");
    assert_eq!(recovered["status"], "ambiguous");
    assert_eq!(recovered["adapter_error_id"], "apply_outcome_ambiguous");
    assert_eq!(recovered["ledger_state"], "ambiguous");
    assert_eq!(recovered["effect_started"], true);
    assert_eq!(recovered["effect"], "none");
    assert_eq!(snapshot(&fixture.root), partial);

    let terminal = fs::read_to_string(fixture.store.root.join("authority-ledger.json")).unwrap();
    assert_eq!(terminal.matches("\"state\":\"reserved\"").count(), 1);
    assert_eq!(terminal.matches("\"state\":\"effect_started\"").count(), 1);
    assert_eq!(terminal.matches("\"state\":\"ambiguous\"").count(), 1);
    assert!(!terminal.contains("committed"));
}

#[test]
fn abrupt_process_exit_after_effect_start_with_exact_existing_ancestor_preimage_is_interrupted() {
    let fixture = Fixture::new("process-crash-existing-ancestor-preimage");
    create_existing_codex_ancestor(&fixture);
    let before = snapshot(&fixture.root);
    let _intent = run_authority_crash(
        &fixture,
        "crash-after-start",
        "crash-existing-ancestor-preimage-owner",
        89,
    );
    assert_eq!(snapshot(&fixture.root), before);

    let recovered = run_authority_scenario(
        &fixture,
        "recover-expired",
        "crash-existing-ancestor-preimage-owner",
    );
    assert_eq!(recovered["status"], "interrupted");
    assert_eq!(recovered["adapter_error_id"], "apply_outcome_ambiguous");
    assert_eq!(recovered["ledger_state"], "interrupted");
    assert_eq!(recovered["effect_started"], true);
    assert_eq!(snapshot(&fixture.root), before);
}

#[test]
fn recovery_intent_parse_and_inspect_are_bounded_canonical_and_zero_write() {
    let fixture = Fixture::new("recovery-intent-parse");
    let context = fixture.context();
    let prepared = fixture.prepared(&context);
    let before_target = snapshot(&fixture.root);
    let before_status = git_status(&fixture.root);
    let before_store = fixture.store_names();

    let intent = prepare_recovery_intent(&context, &prepared).unwrap();
    let bytes = intent.to_machine_bytes();
    let parsed = parse_recovery_intent(&bytes).unwrap();
    assert_eq!(parsed.to_machine_bytes(), bytes);
    assert!(!String::from_utf8_lossy(&bytes).contains(fixture.root.to_str().unwrap()));
    assert!(bytes.len() < 64 * 1024);

    let mut mutated = bytes.clone();
    mutated[0] ^= 1;
    assert_eq!(
        parse_recovery_intent(&mutated).unwrap_err().id(),
        AdapterErrorId::ApplyPermitInvalid
    );
    let mut noncanonical = bytes.clone();
    noncanonical.push(b'\n');
    assert_eq!(
        parse_recovery_intent(&noncanonical).unwrap_err().id(),
        AdapterErrorId::ApplyPermitInvalid
    );
    let oversize = vec![b'x'; 64 * 1024 + 1];
    assert_eq!(
        parse_recovery_intent(&oversize).unwrap_err().id(),
        AdapterErrorId::ApplyPermitInvalid
    );

    assert_eq!(snapshot(&fixture.root), before_target);
    assert_eq!(git_status(&fixture.root), before_status);
    assert_eq!(fixture.store_names(), before_store);
}

#[test]
fn wrong_nonce_and_stale_canonical_intent_refuse_without_terminal_mutation() {
    let fixture = Fixture::new("recovery-intent-binding");
    let intent_path = run_authority_crash(
        &fixture,
        "crash-before",
        "recovery-intent-binding-owner",
        86,
    );
    let intent_bytes = fs::read(intent_path).unwrap();
    let before_target = snapshot(&fixture.root);
    let ledger_path = fixture.store.root.join("authority-ledger.json");
    let before_ledger = fs::read(&ledger_path).unwrap();
    let context = fixture.context();

    let wrong_nonce = recover_prepared_apply(
        &context,
        &intent_bytes,
        &TestClock::new([73]),
        &fixture.store,
        nonce("recovery-intent-wrong-owner"),
    );
    assert_eq!(wrong_nonce.status(), "refused");
    assert_eq!(
        wrong_nonce.error_id(),
        Some(AdapterErrorId::ApplyPermitReplayed)
    );
    assert_eq!(fs::read(&ledger_path).unwrap(), before_ledger);
    assert_eq!(snapshot(&fixture.root), before_target);

    let other = Fixture::new("recovery-intent-stale-source");
    let other_context = other.context();
    let stale_intent = prepare_recovery_intent(&other_context, &other.prepared(&other_context))
        .unwrap()
        .to_machine_bytes();
    let stale = recover_prepared_apply(
        &context,
        &stale_intent,
        &TestClock::new([73]),
        &fixture.store,
        nonce("recovery-intent-binding-owner"),
    );
    assert_eq!(stale.status(), "refused");
    assert_eq!(stale.error_id(), Some(AdapterErrorId::ApplyPermitReplayed));
    assert_eq!(fs::read(&ledger_path).unwrap(), before_ledger);
    assert_eq!(snapshot(&fixture.root), before_target);

    let recovered =
        run_authority_scenario(&fixture, "recover-expired", "recovery-intent-binding-owner");
    assert_eq!(recovered["status"], "interrupted");
}

#[test]
fn subprocess_reservation_helper() {
    if std::env::var_os("HUL_FIT_AUTHORITY_SUBPROCESS").is_none() {
        return;
    }
    let root = PathBuf::from(std::env::var_os("HUL_FIT_AUTHORITY_STORE").unwrap());
    let id = std::env::var("HUL_FIT_AUTHORITY_STORE_ID").unwrap();
    let ledger = FileRepositoryFitLedger::open_or_initialize(&root, &id).unwrap();
    match ledger.reserve(reservation('1')).unwrap() {
        ReservationDecision::Acquired(_) => println!("RESERVATION-WINNER"),
        ReservationDecision::Existing(existing) => {
            assert_eq!(existing.state(), RepositoryFitLedgerState::Reserved);
            assert_eq!(existing.terminal_sha256(), None);
            println!("RESERVATION-EXISTING");
        }
    }
}

#[test]
fn subprocess_authority_scenario_helper() {
    let Some(role) = std::env::var_os("HUL_FIT_AUTHORITY_SCENARIO") else {
        return;
    };
    let role = role.to_string_lossy().into_owned();
    let root = PathBuf::from(std::env::var_os("HUL_FIT_AUTHORITY_REPO").unwrap());
    let store = TestStore {
        root: PathBuf::from(std::env::var_os("HUL_FIT_AUTHORITY_STORE").unwrap()),
        id: std::env::var("HUL_FIT_AUTHORITY_STORE_ID").unwrap(),
    };
    let nonce_label = std::env::var("HUL_FIT_AUTHORITY_NONCE_LABEL").unwrap();
    let intent_path = PathBuf::from(std::env::var_os("HUL_FIT_AUTHORITY_INTENT").unwrap());
    let result = PathBuf::from(std::env::var_os("HUL_FIT_AUTHORITY_RESULT").unwrap());
    let ready = PathBuf::from(std::env::var_os("HUL_FIT_AUTHORITY_READY").unwrap());
    let release = PathBuf::from(std::env::var_os("HUL_FIT_AUTHORITY_RELEASE").unwrap());
    let context = LiveContext::build(BuildRequest::new(&root)).unwrap();

    if matches!(
        role.as_str(),
        "recover-active" | "recover-expired" | "recover-expired-signaled"
    ) {
        if role == "recover-expired-signaled" {
            fs::write(&ready, b"recovery-started\n").unwrap();
        }
        let intent_bytes = fs::read(&intent_path).unwrap();
        let tick = if role == "recover-active" { 22 } else { 73 };
        let outcome = recover_prepared_apply(
            &context,
            &intent_bytes,
            &TestClock::new([tick]),
            &store,
            nonce(&nonce_label),
        );
        fs::write(result, outcome.to_machine_bytes().unwrap()).unwrap();
        return;
    }

    let plan = plan_target(&context).unwrap();
    let prepared = prepare_apply_request(
        &context,
        &plan.to_machine_bytes().unwrap(),
        plan.plan_sha256(),
    )
    .unwrap();
    let recovery_intent = prepare_recovery_intent(&context, &prepared).unwrap();
    let intent_bytes = recovery_intent.to_machine_bytes();
    if intent_path.exists() {
        assert_eq!(fs::read(&intent_path).unwrap(), intent_bytes);
    } else {
        fs::write(&intent_path, &intent_bytes).unwrap();
    }

    match role.as_str() {
        "winner" => after_reservation_for_test(move || {
            fs::write(&ready, b"reserved\n").unwrap();
            wait_for_path(&release);
        }),
        "winner-effect" => after_effect_start_before_apply_for_test(move || {
            fs::write(&ready, b"effect-started\n").unwrap();
            wait_for_path(&release);
        }),
        "contender" => {}
        "expired-execute" => {}
        "crash-before" => after_reservation_for_test(|| unsafe { libc::_exit(86) }),
        "crash-after-start" => {
            after_effect_start_before_apply_for_test(|| unsafe { libc::_exit(89) })
        }
        "crash-after" => after_effect_before_terminal_for_test(|| unsafe { libc::_exit(88) }),
        other => panic!("unknown authority subprocess role: {other}"),
    }

    let ticks = match role.as_str() {
        "contender" => [20, 21, 22],
        "expired-execute" => [71, 72, 73],
        _ => [10, 11, 12],
    };
    let outcome = execute_prepared_apply(
        &context,
        prepared,
        recovery_intent,
        &TestClock::new(ticks),
        &store,
        nonce(&nonce_label),
    );
    fs::write(result, outcome.to_machine_bytes().unwrap()).unwrap();
}

#[test]
fn nonce_and_semantic_replay_and_active_target_lease_fail_closed() {
    let fixture = Fixture::new("replay-lease");
    let ledger =
        FileRepositoryFitLedger::open_or_initialize(&fixture.store.root, fixture.store.store_id())
            .unwrap();
    assert!(matches!(
        ledger.reserve(reservation('1')).unwrap(),
        ReservationDecision::Acquired(_)
    ));
    assert!(matches!(
        ledger.reserve(reservation('1')).unwrap(),
        ReservationDecision::Existing(_)
    ));
    let first = reservation('1');
    let same_target = ReservationRequest {
        binding_sha256: &fixed_digest('a'),
        semantic_effect_id: &fixed_digest('b'),
        target_scope_id: first.target_scope_id,
        permit_id: &fixed_digest('c'),
        nonce_sha256: &fixed_digest('d'),
        recovery_intent_sha256: first.recovery_intent_sha256,
        issued_tick: 10,
        expires_tick: 20,
        recovery: first.recovery,
    };
    assert!(matches!(
        ledger.reserve(same_target),
        Err(ref error) if error.id() == LedgerErrorId::ActiveLease
    ));
    let same_nonce_other_effect = ReservationRequest {
        binding_sha256: &fixed_digest('a'),
        semantic_effect_id: &fixed_digest('b'),
        target_scope_id: &fixed_digest('c'),
        permit_id: &fixed_digest('d'),
        nonce_sha256: first.nonce_sha256,
        recovery_intent_sha256: first.recovery_intent_sha256,
        issued_tick: 10,
        expires_tick: 20,
        recovery: first.recovery,
    };
    assert!(matches!(
        ledger.reserve(same_nonce_other_effect),
        Err(ref error) if error.id() == LedgerErrorId::Replay
    ));

    let mut mismatched_recovery = first.recovery.clone();
    mismatched_recovery.rows[0].post_sha256 = fixed_digest('a');
    let mismatched_intent = ReservationRequest {
        binding_sha256: &fixed_digest('a'),
        semantic_effect_id: &fixed_digest('b'),
        target_scope_id: &fixed_digest('c'),
        permit_id: &fixed_digest('d'),
        nonce_sha256: &fixed_digest('e'),
        recovery_intent_sha256: first.recovery_intent_sha256,
        issued_tick: 10,
        expires_tick: 20,
        recovery: &mismatched_recovery,
    };
    assert!(matches!(
        ledger.reserve(mismatched_intent),
        Err(ref error) if error.id() == LedgerErrorId::InvalidTransition
    ));
}

#[test]
fn production_mutation_grant_has_one_private_mint_in_the_sealed_authority() {
    let source_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/repository_fit");
    let authority = fs::read_to_string(source_root.join("product_adapter/authority.rs")).unwrap();
    let adapter = fs::read_to_string(source_root.join("product_adapter.rs")).unwrap();
    let local = fs::read_to_string(source_root.join("local/mod.rs")).unwrap();
    let effects = fs::read_to_string(source_root.join("local/effects.rs")).unwrap();

    assert_eq!(authority.matches("struct LocalMutationGrant").count(), 1);
    assert_eq!(authority.matches("const fn issue() -> Self").count(), 1);
    assert_eq!(authority.matches("LocalMutationGrant::issue()").count(), 1);
    assert!(!authority.contains("pub(crate) const fn issue"));
    assert!(!authority.contains("pub(super) const fn issue"));
    assert!(!authority.contains("pub(in crate::repository_fit) const fn issue"));
    assert!(adapter.contains("pub(in crate::repository_fit) use authority::LocalMutationGrant;"));
    assert!(!local.contains("LocalMutationGrant"));
    assert!(!local.contains("issue_local_mutation_grant"));
    assert_eq!(
        effects
            .matches("use crate::repository_fit::product_adapter::LocalMutationGrant;")
            .count(),
        2
    );
    assert_eq!(effects.matches("_grant: LocalMutationGrant").count(), 2);
}

#[test]
fn worker_result_is_typed_and_freezes_the_exact_regular_single_link_artifact_set() {
    const RESULT_PATH: &str =
        "docs/ultragoal-successor-live/worker-results/REPOSITORY-FIT-PRODUCTION-AUTHORITY-085.json";
    const ARTIFACT_PATHS: [&str; 10] = [
        "validator/src/repository_fit/product_adapter.rs",
        "validator/src/repository_fit/product_adapter/root_permit.rs",
        "validator/src/repository_fit/product_adapter/authority.rs",
        "validator/src/repository_fit/product_adapter/ledger.rs",
        "validator/src/repository_fit/product_adapter/tests.rs",
        "validator/src/repository_fit/product_adapter/tests/production_authority.rs",
        "validator/src/repository_fit/mod.rs",
        "validator/src/repository_fit/local/mod.rs",
        "validator/src/repository_fit/local/unix.rs",
        "validator/src/repository_fit/local/effects.rs",
    ];
    const R5_TOUCHED_PATHS: [&str; 4] = [
        "validator/src/repository_fit/product_adapter/authority.rs",
        "validator/src/repository_fit/product_adapter/ledger.rs",
        "validator/src/repository_fit/product_adapter/root_permit.rs",
        "validator/src/repository_fit/product_adapter/tests/production_authority.rs",
    ];

    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let result_bytes = fs::read(workspace.join(RESULT_PATH)).unwrap();
    assert!(!result_bytes.is_empty() && result_bytes.len() <= 4 * 1024 * 1024);
    let result: ProofWorkerResultV1 = serde_json::from_slice(&result_bytes).unwrap();
    assert_eq!(
        result.worker,
        "/root/repository_fit_proof_recovery_engineer"
    );
    assert_eq!(
        result.lease_id,
        "REPOSITORY-FIT-ROOT-BINDING-RECOVERY-085-R5"
    );
    assert_eq!(
        result.no_claim_statement,
        "This worker does not claim readiness, release, or completion."
    );
    let status = result
        .final_state
        .get("status")
        .and_then(serde_json::Value::as_str);
    let blocker = result.final_state.get("blocker");
    let current_candidate_proof = result
        .final_state
        .get("current_candidate_proof")
        .and_then(serde_json::Value::as_str);
    match status {
        Some("worker_blocked") => {
            assert_eq!(
                blocker.and_then(serde_json::Value::as_str),
                Some("current_candidate_validation_enospc")
            );
            assert_eq!(current_candidate_proof, Some("not_established"));
        }
        Some("candidate_for_root_acceptance") => {
            assert!(
                blocker.is_none(),
                "accepted candidate cannot retain a blocker"
            );
            assert_eq!(current_candidate_proof, Some("established"));
        }
        other => panic!("unrecognized repository-fit WorkerResult status: {other:?}"),
    }

    let mut touched = result.touched_paths.clone();
    touched.sort();
    let mut expected_touched = R5_TOUCHED_PATHS.map(str::to_owned).to_vec();
    expected_touched.push(RESULT_PATH.to_owned());
    expected_touched.sort();
    assert_eq!(touched, expected_touched);
    assert_eq!(result.generated_outputs, [RESULT_PATH]);
    assert!(result.fixtures.is_empty());

    let artifact_paths = result
        .artifacts
        .iter()
        .map(|artifact| artifact.path.as_str())
        .collect::<Vec<_>>();
    assert_eq!(artifact_paths, ARTIFACT_PATHS);
    for artifact in &result.artifacts {
        let path = workspace.join(&artifact.path);
        let metadata = fs::symlink_metadata(&path).unwrap();
        assert!(
            metadata.is_file(),
            "{} is not a regular file",
            artifact.path
        );
        assert_eq!(metadata.nlink(), 1, "{} is not single-link", artifact.path);
        let bytes = fs::read(path).unwrap();
        assert_eq!(
            bytes.len() as u64,
            artifact.byte_length,
            "{}",
            artifact.path
        );
        assert_eq!(digest(&bytes), artifact.sha256, "{}", artifact.path);
    }

    let mut canonical_artifacts = result.artifacts.iter().collect::<Vec<_>>();
    canonical_artifacts.sort_by(|left, right| left.path.as_bytes().cmp(right.path.as_bytes()));
    let mut canonical_artifact_set = Vec::new();
    for artifact in canonical_artifacts {
        canonical_artifact_set.extend_from_slice(artifact.path.as_bytes());
        canonical_artifact_set.push(b'\t');
        canonical_artifact_set.extend_from_slice(
            artifact
                .sha256
                .strip_prefix("sha256:")
                .expect("artifact digests use the canonical sha256: prefix")
                .as_bytes(),
        );
        canonical_artifact_set.push(b'\n');
    }
    let artifact_set_sha256 = digest(&canonical_artifact_set);
    assert_eq!(
        result
            .candidate_identity
            .get("artifact_set_formula")
            .and_then(serde_json::Value::as_str),
        Some(
            "Sort artifact rows by UTF-8 path bytes, serialize each as path<TAB>lowercase SHA-256 without the sha256: prefix<LF>, concatenate, then SHA-256 the resulting bytes."
        )
    );
    assert_eq!(
        result
            .candidate_identity
            .get("artifact_set_sha256")
            .and_then(serde_json::Value::as_str),
        Some(artifact_set_sha256.as_str())
    );
    eprintln!(
        "worker_result_id={} artifact_set_sha256={artifact_set_sha256}",
        digest(&result_bytes)
    );
}
