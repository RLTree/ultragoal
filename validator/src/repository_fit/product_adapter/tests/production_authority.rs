#![cfg(target_vendor = "apple")]

use super::super::authority::{
    after_reservation_for_test, configure_effects_for_test, execute_prepared_apply,
    RepositoryFitApplyNonce, RepositoryFitAuthorityStore, RepositoryFitTrustedClock,
};
use super::super::catalog::CANONICAL_TEMPLATES;
use super::super::ledger::{
    FileRepositoryFitLedger, LedgerErrorId, RepositoryFitLedgerState, ReservationDecision,
    ReservationRequest,
};
use super::super::{plan_target, prepare_apply_request, verify_target, AdapterErrorId};
use super::support::{git_status, snapshot};
use crate::context::{BuildRequest, LiveContext};
use crate::repository_fit::{digest, inspect_target, FitAdapterError, PreparedFitApply};
use std::collections::VecDeque;
use std::fs;
use std::os::unix::fs::{symlink, MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Barrier, Mutex};

const BASE: &str = "/private/tmp/hul-repository-fit-production-authority-085-fixtures";
static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

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
    execute_prepared_apply(
        context,
        prepared,
        &TestClock::new([10, 11]),
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
    ]));
    ReservationRequest {
        binding_sha256: &values[0],
        semantic_effect_id: &values[1],
        target_scope_id: &values[2],
        permit_id: &values[3],
        nonce_sha256: &values[4],
        issued_tick: 10,
        expires_tick: 20,
    }
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
fn interrupted_reservation_reconciles_to_terminal_without_target_write() {
    let fixture = Fixture::new("interrupted");
    let context = fixture.context();
    let prepared = fixture.prepared(&context);
    let before = snapshot(&fixture.root);
    after_reservation_for_test(|| panic!("simulated process interruption"));
    let interrupted = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        execute(&fixture, &context, prepared, "interrupted-first")
    }));
    assert!(interrupted.is_err());
    assert_eq!(snapshot(&fixture.root), before);

    let retry_context = fixture.context();
    let retry = execute(
        &fixture,
        &retry_context,
        fixture.prepared(&retry_context),
        "interrupted-retry",
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
    stale.write(CANONICAL_TEMPLATES[0].target_path, b"attacker bytes\n");
    let before = snapshot(&stale.root);
    let refused = execute(&stale, &context, prepared, "stale-before-store");
    assert_eq!(refused.status(), "refused");
    assert_eq!(stale.store_names(), Vec::<String>::new());
    assert_eq!(snapshot(&stale.root), before);

    let expired = Fixture::new("clock-before-store");
    let context = expired.context();
    let before = snapshot(&expired.root);
    let result = execute_prepared_apply(
        &context,
        expired.prepared(&context),
        &TestClock::new([100, 99]),
        &expired.store,
        nonce("clock-before-store"),
    );
    assert_eq!(result.error_id(), Some(AdapterErrorId::ApplyPermitExpired));
    assert_eq!(expired.store_names(), Vec::<String>::new());
    assert_eq!(snapshot(&expired.root), before);
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
        let outcome = execute(&fixture, &context, prepared, kind);
        assert_eq!(outcome.status(), "refused");
        assert_eq!(fixture.store_names(), Vec::<String>::new());
    }
}

#[test]
fn canonical_outcome_redacts_nonce_path_secret_and_backend_canaries() {
    let fixture = Fixture::new("redaction");
    let context = fixture.context();
    let canary = "zzq7V5-repository-fit-production-nonce-secret";
    let outcome = execute_prepared_apply(
        &context,
        fixture.prepared(&context),
        &TestClock::new([10, 11]),
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
    after_reservation_for_test(|| panic!("interrupt before effect"));
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        execute(&fixture, &context, prepared, "forged-success-nonce")
    }));
    fixture.install_all_direct();
    let forged_context = fixture.context();
    let forged = execute(
        &fixture,
        &forged_context,
        fixture.prepared(&forged_context),
        "forged-success-nonce",
    );
    assert_eq!(forged.status(), "refused");
    assert_eq!(forged.error_id(), Some(AdapterErrorId::ApplyPermitReplayed));
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
                fs::write(&lock, b"repository-fit-authority-lock-v1\n").unwrap();
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

    for kind in ["symlink-root", "fifo-key"] {
        let fixture = Fixture::new(kind);
        let requested = if kind == "symlink-root" {
            let real = fixture.container.join("real-authority");
            fs::create_dir(&real).unwrap();
            fs::set_permissions(&real, fs::Permissions::from_mode(0o700)).unwrap();
            let alias = fixture.container.join("authority-alias");
            symlink(&real, &alias).unwrap();
            alias
        } else {
            let key = fixture.store.root.join("authority.key");
            let path = std::ffi::CString::new(key.as_os_str().as_encoded_bytes()).unwrap();
            assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
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
fn two_processes_racing_one_nonce_and_semantic_effect_have_one_winner() {
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
        issued_tick: 10,
        expires_tick: 20,
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
        issued_tick: 10,
        expires_tick: 20,
    };
    assert!(matches!(
        ledger.reserve(same_nonce_other_effect),
        Err(ref error) if error.id() == LedgerErrorId::Replay
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
    const ARTIFACT_PATHS: [&str; 9] = [
        "validator/src/repository_fit/product_adapter.rs",
        "validator/src/repository_fit/product_adapter/root_permit.rs",
        "validator/src/repository_fit/product_adapter/authority.rs",
        "validator/src/repository_fit/product_adapter/ledger.rs",
        "validator/src/repository_fit/product_adapter/tests.rs",
        "validator/src/repository_fit/product_adapter/tests/production_authority.rs",
        "validator/src/repository_fit/mod.rs",
        "validator/src/repository_fit/local/mod.rs",
        "validator/src/repository_fit/local/effects.rs",
    ];

    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let result_bytes = fs::read(workspace.join(RESULT_PATH)).unwrap();
    let result = crate::orchestration::WorkerResultV1::parse_json(&result_bytes).unwrap();
    assert_eq!(
        result.worker,
        "/root/repository_fit_production_authority_engineer"
    );
    assert_eq!(result.lease_id, "REPOSITORY-FIT-PRODUCTION-AUTHORITY-085");
    assert_eq!(
        result.no_claim_statement,
        "This worker does not claim readiness, release, or completion."
    );
    assert_eq!(
        result
            .final_state
            .get("status")
            .and_then(serde_json::Value::as_str),
        Some("worker_blocked")
    );
    assert_eq!(
        result
            .final_state
            .get("blocker")
            .and_then(serde_json::Value::as_str),
        Some("current_candidate_validation_enospc")
    );
    assert_eq!(
        result
            .final_state
            .get("current_candidate_proof")
            .and_then(serde_json::Value::as_str),
        Some("not_established")
    );

    let mut touched = result.touched_paths.clone();
    touched.sort();
    let mut expected_touched = ARTIFACT_PATHS.map(str::to_owned).to_vec();
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
        result.result_id().unwrap()
    );
}
