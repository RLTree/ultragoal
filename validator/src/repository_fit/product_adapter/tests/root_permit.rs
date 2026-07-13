use crate::context::{BuildRequest, LiveContext};
use crate::repository_fit::local::LocalEffects;
use crate::repository_fit::product_adapter::catalog::CANONICAL_TEMPLATES;
use crate::repository_fit::product_adapter::protocol::OpaqueFitApplyRequest;
use crate::repository_fit::product_adapter::root_permit::{
    ProtectedCaptureBoundary, ProtectedCapturePhase, ReconciliationTargetPhase,
    RepositoryFitApplyFailure, RepositoryFitApplyOutcome, RepositoryFitPermitEffects,
    TargetCapturePhase, TestRepositoryFitPermitAuthority, apply_with_root_permit,
    assert_protected_capture_hook_consumed_for_test,
    assert_reconciliation_target_hook_consumed_for_test,
    assert_target_capture_hook_consumed_for_test, before_final_green_observation_for_test,
    before_postflight_observation_for_test, duplicate_authorization_for_test,
    permit_seal_stage_for_test, protected_capture_hook_for_test,
    reconciliation_target_hook_for_test, scope_violation_for_test, target_capture_hook_for_test,
};
use crate::repository_fit::product_adapter::{
    AdapterErrorId, plan_target, prepare_apply_request, verify_target,
};
use crate::repository_fit::{
    CanonicalPath, ExpectedContent, FitEffects, FitError, FitErrorId, FitReader, digest,
};
use std::collections::BTreeSet;
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Barrier, Mutex};
use std::thread::JoinHandle;

use super::support::{git_status, snapshot};

const BASE: &str = "/tmp/hul-repository-fit-apply-mediation-058";
const SECRET: &[u8] = b"repository-fit-test-root-authority-secret-material-v1";
static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

struct Fixture {
    container: PathBuf,
    root: PathBuf,
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
        fs::create_dir_all(&root).unwrap();
        git(&root, &["init", "--quiet"]);
        Self { container, root }
    }

    fn context(&self) -> LiveContext {
        LiveContext::build(BuildRequest::new(&self.root)).unwrap()
    }

    fn write(&self, relative: &str, bytes: &[u8]) {
        let path = self.root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }

    fn write_template(&self, target: &str) {
        let row = CANONICAL_TEMPLATES
            .iter()
            .find(|row| row.target_path == target)
            .unwrap();
        self.write(target, row.bytes);
        fs::set_permissions(
            self.root.join(target),
            fs::Permissions::from_mode(row.unix_mode),
        )
        .unwrap();
    }

    fn install_all(&self) {
        for row in CANONICAL_TEMPLATES {
            self.write(row.target_path, row.bytes);
            fs::set_permissions(
                self.root.join(row.target_path),
                fs::Permissions::from_mode(row.unix_mode),
            )
            .unwrap();
        }
    }

    fn request(&self, context: &LiveContext) -> OpaqueFitApplyRequest {
        let record = plan_target(context).unwrap();
        prepare_apply_request(
            context,
            &record.to_machine_bytes().unwrap(),
            record.plan_sha256(),
        )
        .unwrap()
        .into_request()
    }

    fn effects(&self, request: &OpaqueFitApplyRequest) -> LocalEffects {
        LocalEffects::open_for_test(&self.root, request.unix_modes.clone()).unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        debug_assert!(self.container.starts_with(BASE));
        let _ = fs::remove_dir_all(&self.container);
    }
}

fn new_authority() -> TestRepositoryFitPermitAuthority {
    TestRepositoryFitPermitAuthority::new(SECRET).unwrap()
}

fn expect_apply_ok<E: RepositoryFitPermitEffects>(
    result: Result<RepositoryFitApplyOutcome, RepositoryFitApplyFailure<E>>,
) -> RepositoryFitApplyOutcome {
    match result {
        Ok(outcome) => outcome,
        Err(failure) => panic!("apply failed with {:?}", failure.error().id()),
    }
}

fn expect_apply_failure<E: RepositoryFitPermitEffects>(
    result: Result<RepositoryFitApplyOutcome, RepositoryFitApplyFailure<E>>,
) -> RepositoryFitApplyFailure<E> {
    match result {
        Err(failure) => failure,
        Ok(outcome) => panic!("unexpected {} apply outcome", outcome.status()),
    }
}

fn nonce(label: &str) -> Vec<u8> {
    digest(label.as_bytes()).into_bytes()
}

fn git(root: &Path, arguments: &[&str]) {
    let status = Command::new("git")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .args(arguments)
        .current_dir(root)
        .status()
        .unwrap();
    assert!(status.success());
}

fn catalog_ancestor_paths() -> BTreeSet<PathBuf> {
    let mut ancestors = BTreeSet::new();
    for row in CANONICAL_TEMPLATES {
        let mut parent = Path::new(row.target_path).parent();
        while let Some(path) = parent {
            if path.as_os_str().is_empty() {
                break;
            }
            ancestors.insert(path.to_path_buf());
            parent = path.parent();
        }
    }
    ancestors
}

fn replace_directory_preserving_children(path: &Path, displaced: &Path) {
    fs::rename(path, displaced).unwrap();
    fs::create_dir(path).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    for entry in fs::read_dir(displaced).unwrap() {
        let entry = entry.unwrap();
        fs::rename(entry.path(), path.join(entry.file_name())).unwrap();
    }
}

fn replace_directory_with_fifo(path: &Path, displaced: &Path) {
    fs::rename(path, displaced).unwrap();
    let encoded = std::ffi::CString::new(path.as_os_str().as_encoded_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(encoded.as_ptr(), 0o600) }, 0);
}

fn arm_capture_barrier(
    phase: TargetCapturePhase,
    path: &str,
    mutate: impl FnOnce() + Send + 'static,
) -> JoinHandle<()> {
    let reached = Arc::new(Barrier::new(2));
    let resume = Arc::new(Barrier::new(2));
    let attacker_reached = Arc::clone(&reached);
    let attacker_resume = Arc::clone(&resume);
    let attacker = std::thread::spawn(move || {
        attacker_reached.wait();
        mutate();
        attacker_resume.wait();
    });
    target_capture_hook_for_test(phase, path, move || {
        reached.wait();
        resume.wait();
    });
    attacker
}

fn arm_alternating_target_hybrid(
    first_path: &'static str,
    second_path: &'static str,
    first: PathBuf,
    second: PathBuf,
    first_desired: Vec<u8>,
    first_other: Vec<u8>,
    second_desired: Vec<u8>,
    second_other: Vec<u8>,
    remaining_collects: usize,
) {
    if remaining_collects == 0 {
        return;
    }
    target_capture_hook_for_test(
        TargetCapturePhase::AfterLeafRevalidated,
        first_path,
        move || {
            fs::write(&first, &first_other).unwrap();
            fs::write(&second, &second_other).unwrap();
            target_capture_hook_for_test(
                TargetCapturePhase::AfterLeafRevalidated,
                second_path,
                move || {
                    fs::write(&second, &second_desired).unwrap();
                    fs::write(&first, &first_desired).unwrap();
                    arm_alternating_target_hybrid(
                        first_path,
                        second_path,
                        first,
                        second,
                        first_desired,
                        first_other,
                        second_desired,
                        second_other,
                        remaining_collects - 1,
                    );
                },
            );
        },
    );
}

fn same_length_other(bytes: &[u8]) -> Vec<u8> {
    let mut other = bytes.to_vec();
    let byte = other
        .first_mut()
        .expect("canonical target fixtures are nonempty");
    *byte ^= 1;
    other
}

fn assert_target_complete_collect_race(
    label: &str,
    mutate: impl FnOnce(&Path, &Path, &Path) + 'static,
) {
    let fixture = Fixture::new(label);
    fixture.install_all();
    let context = fixture.context();
    let request = fixture.request(&context);
    let target = fixture.root.join("AGENTS.md");
    let root = fixture.root.clone();
    let container = fixture.container.clone();
    target_capture_hook_for_test(TargetCapturePhase::BeforeFinalChainRecheck, "", move || {
        mutate(&target, &root, &container)
    });
    let result = new_authority().issue(
        &context,
        &request,
        fixture.effects(&request),
        10,
        20,
        &nonce(label),
    );
    assert_target_capture_hook_consumed_for_test();
    let failure = match result {
        Err(failure) => failure,
        Ok(_) => panic!("a {label} target race unexpectedly issued authority"),
    };
    assert_eq!(failure.id(), AdapterErrorId::TargetUnavailable, "{label}");
}

fn alternate_supplementary_gid(current: u32) -> u32 {
    let count = unsafe { libc::getgroups(0, std::ptr::null_mut()) };
    assert!(count > 0, "the target-race test requires a local group");
    let mut groups = vec![0 as libc::gid_t; count as usize];
    assert_eq!(
        unsafe { libc::getgroups(count, groups.as_mut_ptr()) },
        count
    );
    groups
        .into_iter()
        .map(|group| group as u32)
        .find(|group| *group != current)
        .expect("the target-race test requires an alternate supplementary group")
}

fn arm_protected_capture_barrier(
    boundary: ProtectedCaptureBoundary,
    phase: ProtectedCapturePhase,
    path: &[u8],
    mutate: impl FnOnce() + Send + 'static,
) -> JoinHandle<()> {
    let reached = Arc::new(Barrier::new(2));
    let resume = Arc::new(Barrier::new(2));
    let attacker_reached = Arc::clone(&reached);
    let attacker_resume = Arc::clone(&resume);
    let attacker = std::thread::spawn(move || {
        attacker_reached.wait();
        mutate();
        attacker_resume.wait();
    });
    protected_capture_hook_for_test(boundary, phase, path, move || {
        reached.wait();
        resume.wait();
    });
    attacker
}

fn protected_directory_swap(fixture: &Fixture, label: &str) -> (PathBuf, PathBuf, PathBuf) {
    let active = fixture.root.join("private");
    let replacement = fixture.container.join(format!("{label}-protected-b"));
    let displaced = fixture.container.join(format!("{label}-protected-a"));
    fs::create_dir_all(&active).unwrap();
    fs::write(active.join("preserved.txt"), b"protected A bytes\n").unwrap();
    fs::create_dir_all(&replacement).unwrap();
    fs::write(replacement.join("preserved.txt"), b"protected A bytes\n").unwrap();
    (active, replacement, displaced)
}

fn swap_protected_directories(active: PathBuf, replacement: PathBuf, displaced: PathBuf) {
    fs::rename(&active, displaced).unwrap();
    fs::rename(replacement, active).unwrap();
}

fn change_version(path: &Path) -> (i64, i64) {
    let metadata = fs::metadata(path).unwrap();
    (metadata.ctime(), metadata.ctime_nsec())
}

fn assert_late_regular_write_remains(
    path: &Path,
    expected_inode: u64,
    expected_bytes: &[u8],
    original_version: (i64, i64),
) {
    let metadata = fs::metadata(path).unwrap();
    assert_eq!(metadata.ino(), expected_inode);
    assert_eq!(fs::read(path).unwrap(), expected_bytes);
    assert_ne!(change_version(path), original_version);
}

fn assert_pre_effect_ancestor_drift(
    label: &str,
    ancestor: &str,
    mutate: impl FnOnce(&Fixture, &Path),
) {
    let fixture = Fixture::new(label);
    let path = fixture.root.join(ancestor);
    fs::create_dir_all(&path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    let context = fixture.context();
    let request = fixture.request(&context);
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            fixture.effects(&request),
            10,
            20,
            &nonce(label),
        )
        .unwrap();
    mutate(&fixture, &path);
    let failure = expect_apply_failure(apply_with_root_permit(
        &context,
        request,
        Some(permit),
        Some(lease),
        10,
    ));
    assert!(!failure.effect_started(), "{label}");
}

fn assert_final_green_ancestor_drift(
    label: &str,
    ancestor: &str,
    mutate: impl FnOnce(PathBuf, PathBuf) + 'static,
) {
    let fixture = Fixture::new(label);
    let path = fixture.root.join(ancestor);
    fs::create_dir_all(&path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    let context = fixture.context();
    let request = fixture.request(&context);
    assert!(!request.plan.mutations.is_empty());
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            fixture.effects(&request),
            10,
            20,
            &nonce(label),
        )
        .unwrap();
    let displaced = fixture.container.join(format!("{label}-displaced"));
    before_final_green_observation_for_test(move || mutate(path, displaced));
    let failure = expect_apply_failure(apply_with_root_permit(
        &context,
        request,
        Some(permit),
        Some(lease),
        10,
    ));
    assert_eq!(failure.error().id(), AdapterErrorId::ApplyOutcomeAmbiguous);
    assert!(failure.effect_started(), "{label}");
    assert!(!failure.rollback_complete(), "{label}");
}

#[test]
fn fresh_partial_dirty_and_idempotent_apply_are_exact_and_causal() {
    let fresh = Fixture::new("fresh-and-repeat");
    let context = fresh.context();
    let request = fresh.request(&context);
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            fresh.effects(&request),
            10,
            20,
            &nonce("fresh"),
        )
        .unwrap();
    let outcome = expect_apply_ok(apply_with_root_permit(
        &context,
        request,
        Some(permit),
        Some(lease),
        10,
    ));
    assert_eq!(outcome.status(), "applied");
    assert_eq!(outcome.mutation_count(), CANONICAL_TEMPLATES.len());
    assert!(outcome.outcome_id().starts_with("sha256:"));
    assert!(verify_target(&fresh.context()).unwrap().idempotent());

    let repeated_context = fresh.context();
    let repeated = fresh.request(&repeated_context);
    assert!(repeated.plan.mutations.is_empty());
    let repeated_authority = new_authority();
    let (permit, lease) = repeated_authority
        .issue(
            &repeated_context,
            &repeated,
            fresh.effects(&repeated),
            30,
            40,
            &nonce("repeat"),
        )
        .unwrap();
    let repeated_outcome = expect_apply_ok(apply_with_root_permit(
        &repeated_context,
        repeated,
        Some(permit),
        Some(lease),
        30,
    ));
    assert_eq!(repeated_outcome.status(), "idempotent");
    assert_eq!(repeated_outcome.mutation_count(), 0);

    let partial = Fixture::new("partial-dirty");
    partial.write_template("AGENTS.md");
    partial.write(
        "private/unrelated.txt",
        b"preserve this exact dirty state\n",
    );
    let unrelated = fs::read(partial.root.join("private/unrelated.txt")).unwrap();
    let context = partial.context();
    assert!(context.candidate().dirty);
    let request = partial.request(&context);
    assert_eq!(request.plan.mutations.len(), CANONICAL_TEMPLATES.len() - 1);
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            partial.effects(&request),
            50,
            60,
            &nonce("partial-dirty"),
        )
        .unwrap();
    let outcome = expect_apply_ok(apply_with_root_permit(
        &context,
        request,
        Some(permit),
        Some(lease),
        50,
    ));
    assert_eq!(outcome.status(), "applied");
    assert_eq!(
        fs::read(partial.root.join("private/unrelated.txt")).unwrap(),
        unrelated
    );
}

#[test]
fn missing_managed_ancestors_are_created_with_exact_root_ownership_and_mode() {
    let fixture = Fixture::new("created-managed-ancestors");
    let context = fixture.context();
    let request = fixture.request(&context);
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            fixture.effects(&request),
            10,
            20,
            &nonce("created-managed-ancestors"),
        )
        .unwrap();
    let outcome = expect_apply_ok(apply_with_root_permit(
        &context,
        request,
        Some(permit),
        Some(lease),
        10,
    ));
    assert_eq!(outcome.status(), "applied");
    let root = fs::symlink_metadata(&fixture.root).unwrap();
    let ancestors = catalog_ancestor_paths();
    assert!(ancestors.len() >= 8);
    for ancestor in ancestors {
        let metadata = fs::symlink_metadata(fixture.root.join(&ancestor)).unwrap();
        assert!(metadata.is_dir(), "{}", ancestor.display());
        assert_eq!(metadata.dev(), root.dev(), "{}", ancestor.display());
        assert_eq!(metadata.uid(), root.uid(), "{}", ancestor.display());
        assert_eq!(metadata.gid(), root.gid(), "{}", ancestor.display());
        assert_eq!(metadata.mode() & 0o7777, 0o755, "{}", ancestor.display());
    }
    assert!(verify_target(&fixture.context()).unwrap().idempotent());
}

#[test]
fn missing_permit_missing_lease_and_time_windows_refuse_before_effect_without_consumption() {
    let fixture = Fixture::new("missing-permit");
    let context = fixture.context();
    let request = fixture.request(&context);
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            fixture.effects(&request),
            10,
            20,
            &nonce("missing"),
        )
        .unwrap();
    let before = snapshot(&fixture.root);
    let failure = expect_apply_failure(apply_with_root_permit(
        &context,
        request,
        None,
        Some(lease),
        10,
    ));
    assert_eq!(failure.error().id(), AdapterErrorId::ApplyPermitMissing);
    assert!(!failure.effect_started());
    let pre = failure.into_pre_effect().unwrap();
    let (request, missing, lease) = pre.into_parts();
    assert!(missing.is_none());
    assert_eq!(permit_seal_stage_for_test(&request), 0);
    assert_eq!(snapshot(&fixture.root), before);
    let outcome = expect_apply_ok(apply_with_root_permit(
        &context,
        request,
        Some(permit),
        lease,
        10,
    ));
    assert_eq!(outcome.status(), "applied");

    let fixture = Fixture::new("missing-lease");
    let context = fixture.context();
    let request = fixture.request(&context);
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            fixture.effects(&request),
            50,
            60,
            &nonce("missing-lease"),
        )
        .unwrap();
    let before = snapshot(&fixture.root);
    let failure = expect_apply_failure(apply_with_root_permit::<LocalEffects>(
        &context,
        request,
        Some(permit),
        None,
        50,
    ));
    assert_eq!(failure.error().id(), AdapterErrorId::ApplyLeaseInvalid);
    assert!(!failure.effect_started());
    let (request, permit, missing) = failure.into_pre_effect().unwrap().into_parts();
    assert!(missing.is_none());
    assert_eq!(snapshot(&fixture.root), before);
    let outcome = expect_apply_ok(apply_with_root_permit(
        &context,
        request,
        permit,
        Some(lease),
        50,
    ));
    assert_eq!(outcome.status(), "applied");

    let fixture = Fixture::new("not-yet-valid");
    let context = fixture.context();
    let request = fixture.request(&context);
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            fixture.effects(&request),
            100,
            110,
            &nonce("not-yet-valid"),
        )
        .unwrap();
    let failure = expect_apply_failure(apply_with_root_permit(
        &context,
        request,
        Some(permit),
        Some(lease),
        99,
    ));
    assert_eq!(failure.error().id(), AdapterErrorId::ApplyPermitExpired);
    assert!(!failure.effect_started());
    let (request, permit, lease) = failure.into_pre_effect().unwrap().into_parts();
    assert_eq!(permit_seal_stage_for_test(&request), 0);
    let outcome = expect_apply_ok(apply_with_root_permit(
        &context, request, permit, lease, 100,
    ));
    assert_eq!(outcome.status(), "applied");

    let fixture = Fixture::new("expired");
    let context = fixture.context();
    let request = fixture.request(&context);
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            fixture.effects(&request),
            200,
            210,
            &nonce("expired"),
        )
        .unwrap();
    let before = snapshot(&fixture.root);
    let failure = expect_apply_failure(apply_with_root_permit(
        &context,
        request,
        Some(permit),
        Some(lease),
        211,
    ));
    assert_eq!(failure.error().id(), AdapterErrorId::ApplyPermitExpired);
    assert!(!failure.effect_started());
    assert_eq!(snapshot(&fixture.root), before);
}

#[test]
fn permit_and_mutation_lease_must_share_the_exact_request_and_authority_instance() {
    let fixture = Fixture::new("lease-mismatch");
    let context = fixture.context();
    let request = fixture.request(&context);
    let other_request = fixture.request(&context);
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            fixture.effects(&request),
            10,
            20,
            &nonce("lease-match"),
        )
        .unwrap();
    let other_authority = new_authority();
    let (_other_permit, other_lease) = other_authority
        .issue(
            &context,
            &other_request,
            fixture.effects(&other_request),
            10,
            20,
            &nonce("lease-mismatch"),
        )
        .unwrap();
    let before = snapshot(&fixture.root);
    let failure = expect_apply_failure(apply_with_root_permit(
        &context,
        request,
        Some(permit),
        Some(other_lease),
        10,
    ));
    assert_eq!(failure.error().id(), AdapterErrorId::ApplyLeaseInvalid);
    assert!(!failure.effect_started());
    let (request, permit, _) = failure.into_pre_effect().unwrap().into_parts();
    assert_eq!(snapshot(&fixture.root), before);
    let outcome = expect_apply_ok(apply_with_root_permit(
        &context,
        request,
        permit,
        Some(lease),
        10,
    ));
    assert_eq!(outcome.status(), "applied");
}

#[test]
fn structurally_equal_cross_session_request_cannot_consume_another_request_permit() {
    let fixture = Fixture::new("cross-session");
    let context = fixture.context();
    let request = fixture.request(&context);
    let substituted = fixture.request(&context);
    assert_eq!(request.plan.plan_sha256, substituted.plan.plan_sha256);
    assert_ne!(request.request_id, substituted.request_id);
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            fixture.effects(&request),
            10,
            20,
            &nonce("cross-session"),
        )
        .unwrap();
    let failure = expect_apply_failure(apply_with_root_permit(
        &context,
        substituted,
        Some(permit),
        Some(lease),
        10,
    ));
    assert_eq!(failure.error().id(), AdapterErrorId::ApplyPermitInvalid);
    assert!(!failure.effect_started());
    let (_substituted, permit, lease) = failure.into_pre_effect().unwrap().into_parts();
    assert_eq!(permit_seal_stage_for_test(&request), 0);
    let outcome = expect_apply_ok(apply_with_root_permit(&context, request, permit, lease, 10));
    assert_eq!(outcome.status(), "applied");
}

#[test]
fn concurrent_identical_contenders_have_one_atomic_winner() {
    let fixture = Fixture::new("concurrent-one-winner");
    fixture.install_all();
    let context = fixture.context();
    let request = fixture.request(&context);
    assert!(request.plan.mutations.is_empty());
    let duplicate_request = request.duplicate_for_test();
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            fixture.effects(&request),
            10,
            20,
            &nonce("concurrent"),
        )
        .unwrap();
    let (duplicate_permit, duplicate_lease) = duplicate_authorization_for_test(
        &permit,
        &duplicate_request,
        fixture.effects(&duplicate_request),
    );
    let start = Arc::new(Barrier::new(3));
    let first_start = Arc::clone(&start);
    let second_start = Arc::clone(&start);
    let first_context = context.clone();
    let second_context = context.clone();
    let first = std::thread::spawn(move || {
        first_start.wait();
        apply_with_root_permit(&first_context, request, Some(permit), Some(lease), 10)
    });
    let second = std::thread::spawn(move || {
        second_start.wait();
        apply_with_root_permit(
            &second_context,
            duplicate_request,
            Some(duplicate_permit),
            Some(duplicate_lease),
            10,
        )
    });
    start.wait();
    let results = [first.join().unwrap(), second.join().unwrap()];
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    let loser = results
        .into_iter()
        .find_map(Result::err)
        .expect("one contender must lose");
    assert_eq!(loser.error().id(), AdapterErrorId::ApplyPermitReplayed);
    assert!(loser.effect_started());
}

fn assert_request_tamper_rejected(label: &str, mutate: impl FnOnce(&mut OpaqueFitApplyRequest)) {
    let fixture = Fixture::new(label);
    let context = fixture.context();
    let mut request = fixture.request(&context);
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            fixture.effects(&request),
            10,
            20,
            &nonce(label),
        )
        .unwrap();
    mutate(&mut request);
    let before = snapshot(&fixture.root);
    let failure = expect_apply_failure(apply_with_root_permit(
        &context,
        request,
        Some(permit),
        Some(lease),
        10,
    ));
    assert!(!failure.effect_started(), "{label}");
    assert_ne!(
        failure.error().id(),
        AdapterErrorId::ApplyRolledBack,
        "{label}"
    );
    assert_eq!(snapshot(&fixture.root), before, "{label}");
}

#[test]
fn accepted_plan_desired_state_source_set_modes_and_paths_are_individually_bound() {
    assert_request_tamper_rejected("accepted-digest", |request| {
        request.accepted_plan_sha256 = digest(b"substituted accepted plan");
    });
    assert_request_tamper_rejected("plan-mutation", |request| {
        request.plan.mutations[0].replacement.push(b'x');
    });
    assert_request_tamper_rejected("desired-state", |request| {
        request.desired.files[0].bytes.push(b'x');
    });
    assert_request_tamper_rejected("source-row", |request| {
        request.authority.rows[0].sha256 = digest(b"substituted template source");
    });
    assert_request_tamper_rejected("catalog", |request| {
        request.authority.catalog_sha256 = digest(b"substituted catalog");
    });
    assert_request_tamper_rejected("mode", |request| {
        let path = request.plan.mutations[0].path.as_str().to_owned();
        request.unix_modes.insert(path, 0o600);
    });
    assert_request_tamper_rejected("case-alias", |request| {
        request.plan.mutations[0].path = CanonicalPath::parse("Agents.md").unwrap();
    });
}

#[test]
fn request_identity_context_root_binding_and_canonical_record_are_individually_bound() {
    assert_request_tamper_rejected("request-id", |request| {
        request.request_id = digest(b"substituted request identity");
    });
    assert_request_tamper_rejected("context-id", |request| {
        request.context_id = digest(b"substituted context identity");
    });
    assert_request_tamper_rejected("candidate-id", |request| {
        request.candidate_id = digest(b"substituted candidate identity");
    });
    assert_request_tamper_rejected("root-binding", |request| {
        request.root_binding = digest(b"substituted descriptor root binding");
    });
    assert_request_tamper_rejected("canonical-plan-record", |request| {
        request.plan_record_bytes.push(b' ');
    });
}

#[test]
fn target_projection_dimensions_are_individually_bound() {
    assert_request_tamper_rejected("target-context", |request| {
        request.target.context_id = digest(b"substituted target context");
    });
    assert_request_tamper_rejected("repository-root-id", |request| {
        request.target.repository_root_id = digest(b"substituted repository root");
    });
    assert_request_tamper_rejected("worktree-root-id", |request| {
        request.target.worktree_root_id = digest(b"substituted worktree root");
    });
    assert_request_tamper_rejected("target-candidate-id", |request| {
        request.target.candidate.candidate_id = digest(b"substituted target candidate");
    });
}

#[test]
fn source_manifest_authority_observed_modes_and_desired_digest_are_individually_bound() {
    assert_request_tamper_rejected("manifest", |request| {
        request.authority.manifest_sha256 = digest(b"substituted source manifest");
    });
    assert_request_tamper_rejected("authority", |request| {
        request.authority.authority_sha256 = digest(b"substituted source authority");
    });
    assert_request_tamper_rejected("observed-mode", |request| {
        let path = request.plan.mutations[0].path.as_str().to_owned();
        request.observed_modes.insert(path, Some(0o777));
    });
    assert_request_tamper_rejected("desired-state-digest", |request| {
        request.desired.state_sha256 = digest(b"substituted desired state digest");
    });
}

#[test]
fn same_content_target_replacement_permissions_git_state_and_root_replacement_are_stale() {
    let target = Fixture::new("target-replacement");
    target.write_template("AGENTS.md");
    let context = target.context();
    let request = target.request(&context);
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            target.effects(&request),
            10,
            20,
            &nonce("target-replacement"),
        )
        .unwrap();
    let original = target.root.join("AGENTS.md");
    let replacement = target.root.join("replacement.tmp");
    fs::write(&replacement, fs::read(&original).unwrap()).unwrap();
    fs::set_permissions(&replacement, fs::Permissions::from_mode(0o644)).unwrap();
    fs::rename(&replacement, &original).unwrap();
    let failure = expect_apply_failure(apply_with_root_permit(
        &context,
        request,
        Some(permit),
        Some(lease),
        10,
    ));
    assert!(!failure.effect_started());

    let permissions = Fixture::new("permission-stale");
    permissions.write_template("AGENTS.md");
    let context = permissions.context();
    let request = permissions.request(&context);
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            permissions.effects(&request),
            10,
            20,
            &nonce("permission-stale"),
        )
        .unwrap();
    fs::set_permissions(
        permissions.root.join("AGENTS.md"),
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    let failure = expect_apply_failure(apply_with_root_permit(
        &context,
        request,
        Some(permit),
        Some(lease),
        10,
    ));
    assert!(!failure.effect_started());

    let git_mutation = Fixture::new("git-state-stale");
    let context = git_mutation.context();
    let request = git_mutation.request(&context);
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            git_mutation.effects(&request),
            10,
            20,
            &nonce("git-state-stale"),
        )
        .unwrap();
    let config = git_mutation.root.join(".git/config");
    let mut bytes = fs::read(&config).unwrap();
    bytes.extend_from_slice(b"\n[fit-test]\n\tchanged = true\n");
    fs::write(&config, bytes).unwrap();
    let failure = expect_apply_failure(apply_with_root_permit(
        &context,
        request,
        Some(permit),
        Some(lease),
        10,
    ));
    assert!(!failure.effect_started());

    let root_swap = Fixture::new("root-replacement");
    let context = root_swap.context();
    let request = root_swap.request(&context);
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            root_swap.effects(&request),
            10,
            20,
            &nonce("root-replacement"),
        )
        .unwrap();
    let displaced = root_swap.container.join("displaced-repo");
    fs::rename(&root_swap.root, &displaced).unwrap();
    fs::create_dir(&root_swap.root).unwrap();
    git(&root_swap.root, &["init", "--quiet"]);
    let failure = expect_apply_failure(apply_with_root_permit(
        &context,
        request,
        Some(permit),
        Some(lease),
        10,
    ));
    assert!(!failure.effect_started());
}

#[test]
fn every_existing_managed_ancestor_is_bound_before_effect() {
    assert_pre_effect_ancestor_drift(
        "unique-ancestor-replacement-before-effect",
        "validation_artifacts/harness",
        |fixture, path| {
            replace_directory_preserving_children(
                path,
                &fixture.container.join("unique-ancestor-original"),
            );
        },
    );
    assert_pre_effect_ancestor_drift(
        "shared-ancestor-replacement-before-effect",
        "agent-standards",
        |fixture, path| {
            replace_directory_preserving_children(
                path,
                &fixture.container.join("shared-ancestor-original"),
            );
        },
    );
    assert_pre_effect_ancestor_drift("ancestor-mode-before-effect", ".harness", |_, path| {
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
    });
    assert_pre_effect_ancestor_drift(
        "ancestor-symlink-before-effect",
        ".codex/environments",
        |fixture, path| {
            let displaced = fixture.container.join("ancestor-symlink-original");
            fs::rename(path, &displaced).unwrap();
            symlink(&displaced, path).unwrap();
        },
    );
    assert_pre_effect_ancestor_drift(
        "ancestor-special-before-effect",
        "validation_artifacts/harness",
        |fixture, path| {
            replace_directory_with_fifo(path, &fixture.container.join("ancestor-special-original"));
        },
    );
    assert_pre_effect_ancestor_drift(
        "ancestor-case-alias-before-effect",
        "agent-standards",
        |_, path| {
            fs::rename(path, path.parent().unwrap().join("Agent-Standards")).unwrap();
        },
    );
}

#[test]
fn descriptor_capture_rejects_parent_leaf_and_missing_boundary_hybrids_without_effects() {
    let parent = Fixture::new("descriptor-parent-hybrid");
    parent.install_all();
    let context = parent.context();
    let request = parent.request(&context);
    let before = CANONICAL_TEMPLATES
        .iter()
        .map(|row| {
            (
                row.target_path,
                fs::read(parent.root.join(row.target_path)).unwrap(),
            )
        })
        .collect::<Vec<_>>();
    let active = parent.root.join("agent-standards");
    let displaced = parent.container.join("descriptor-parent-a");
    let attacker = arm_capture_barrier(
        TargetCapturePhase::AfterParentHeldBeforeDescend,
        "agent-standards",
        move || replace_directory_preserving_children(&active, &displaced),
    );
    let failure = match new_authority().issue(
        &context,
        &request,
        parent.effects(&request),
        10,
        20,
        &nonce("descriptor-parent-hybrid"),
    ) {
        Err(failure) => failure,
        Ok(_) => panic!("a parent A/B hybrid unexpectedly issued authority"),
    };
    attacker.join().unwrap();
    assert_target_capture_hook_consumed_for_test();
    assert_eq!(failure.id(), AdapterErrorId::TargetUnavailable);
    for (path, bytes) in before {
        assert_eq!(fs::read(parent.root.join(path)).unwrap(), bytes, "{path}");
    }

    let named = Fixture::new("descriptor-named-parent-hybrid");
    named.install_all();
    let context = named.context();
    let request = named.request(&context);
    let active = named.root.join("agent-standards");
    let displaced = named.container.join("descriptor-named-parent-a");
    let attacker = arm_capture_barrier(
        TargetCapturePhase::AfterNamedBeforeOpen,
        "agent-standards",
        move || replace_directory_preserving_children(&active, &displaced),
    );
    let failure = match new_authority().issue(
        &context,
        &request,
        named.effects(&request),
        10,
        20,
        &nonce("descriptor-named-parent-hybrid"),
    ) {
        Err(failure) => failure,
        Ok(_) => panic!("a named-to-open parent hybrid unexpectedly issued authority"),
    };
    attacker.join().unwrap();
    assert_target_capture_hook_consumed_for_test();
    assert_eq!(failure.id(), AdapterErrorId::TargetUnavailable);

    let leaf = Fixture::new("descriptor-leaf-hybrid");
    leaf.install_all();
    let context = leaf.context();
    let request = leaf.request(&context);
    let active = leaf.root.join("AGENTS.md");
    let displaced = leaf.container.join("descriptor-leaf-a");
    let bytes = fs::read(&active).unwrap();
    let attacker = arm_capture_barrier(
        TargetCapturePhase::AfterLeafHeldBeforeRead,
        "AGENTS.md",
        move || {
            fs::rename(&active, displaced).unwrap();
            fs::write(&active, &bytes).unwrap();
            fs::set_permissions(&active, fs::Permissions::from_mode(0o644)).unwrap();
        },
    );
    let failure = match new_authority().issue(
        &context,
        &request,
        leaf.effects(&request),
        10,
        20,
        &nonce("descriptor-leaf-hybrid"),
    ) {
        Err(failure) => failure,
        Ok(_) => panic!("a leaf A/B hybrid unexpectedly issued authority"),
    };
    attacker.join().unwrap();
    assert_target_capture_hook_consumed_for_test();
    assert_eq!(failure.id(), AdapterErrorId::TargetUnavailable);

    let shared = Fixture::new("descriptor-shared-ancestor-race");
    shared.install_all();
    let context = shared.context();
    let request = shared.request(&context);
    let active = shared.root.join("agent-standards");
    let displaced = shared.container.join("descriptor-shared-a");
    let attacker =
        arm_capture_barrier(TargetCapturePhase::BeforeFinalChainRecheck, "", move || {
            replace_directory_preserving_children(&active, &displaced)
        });
    let failure = match new_authority().issue(
        &context,
        &request,
        shared.effects(&request),
        10,
        20,
        &nonce("descriptor-shared-ancestor-race"),
    ) {
        Err(failure) => failure,
        Ok(_) => panic!("a shared-ancestor final-recheck race unexpectedly issued authority"),
    };
    attacker.join().unwrap();
    assert_target_capture_hook_consumed_for_test();
    assert_eq!(failure.id(), AdapterErrorId::TargetUnavailable);

    let aliased = Fixture::new("descriptor-case-alias-race");
    aliased.install_all();
    let context = aliased.context();
    let request = aliased.request(&context);
    let active = aliased.root.join("agent-standards");
    let alias = aliased.root.join("Agent-Standards");
    let attacker = arm_capture_barrier(
        TargetCapturePhase::AfterNamedBeforeOpen,
        "agent-standards",
        move || fs::rename(active, alias).unwrap(),
    );
    let failure = match new_authority().issue(
        &context,
        &request,
        aliased.effects(&request),
        10,
        20,
        &nonce("descriptor-case-alias-race"),
    ) {
        Err(failure) => failure,
        Ok(_) => panic!("a within-capture case alias unexpectedly issued authority"),
    };
    attacker.join().unwrap();
    assert_target_capture_hook_consumed_for_test();
    assert_eq!(failure.id(), AdapterErrorId::TargetUnavailable);

    let missing = Fixture::new("descriptor-missing-race");
    let context = missing.context();
    let request = missing.request(&context);
    let created = missing.root.join("agent-standards");
    let attacker = arm_capture_barrier(
        TargetCapturePhase::AfterMissingBeforeRecheck,
        "agent-standards",
        move || {
            fs::create_dir(&created).unwrap();
            fs::set_permissions(created, fs::Permissions::from_mode(0o755)).unwrap();
        },
    );
    let failure = match new_authority().issue(
        &context,
        &request,
        missing.effects(&request),
        10,
        20,
        &nonce("descriptor-missing-race"),
    ) {
        Err(failure) => failure,
        Ok(_) => panic!("a missing-boundary race unexpectedly issued authority"),
    };
    attacker.join().unwrap();
    assert_target_capture_hook_consumed_for_test();
    assert_eq!(failure.id(), AdapterErrorId::TargetUnavailable);
    assert!(
        CANONICAL_TEMPLATES
            .iter()
            .all(|row| !missing.root.join(row.target_path).is_file())
    );
}

#[test]
fn target_complete_double_collect_rejects_two_leaf_alternating_hybrid() {
    let fixture = Fixture::new("target-two-leaf-alternating-hybrid");
    fixture.install_all();
    let first_row = &CANONICAL_TEMPLATES[0];
    let second_row = &CANONICAL_TEMPLATES[1];
    let first = fixture.root.join(first_row.target_path);
    let second = fixture.root.join(second_row.target_path);
    let first_other = same_length_other(first_row.bytes);
    let second_other = same_length_other(second_row.bytes);
    let first_version = change_version(&first);
    let second_version = change_version(&second);
    let context = fixture.context();
    let request = fixture.request(&context);
    arm_alternating_target_hybrid(
        first_row.target_path,
        second_row.target_path,
        first.clone(),
        second.clone(),
        first_row.bytes.to_vec(),
        first_other,
        second_row.bytes.to_vec(),
        second_other.clone(),
        2,
    );
    let result = new_authority().issue(
        &context,
        &request,
        fixture.effects(&request),
        10,
        20,
        &nonce("target-two-leaf-alternating-hybrid"),
    );
    assert_target_capture_hook_consumed_for_test();
    let failure = match result {
        Err(failure) => failure,
        Ok(_) => panic!("a two-leaf target hybrid unexpectedly issued authority"),
    };
    assert_eq!(failure.id(), AdapterErrorId::TargetUnavailable);
    assert_eq!(fs::read(&first).unwrap(), first_row.bytes);
    assert_eq!(fs::read(&second).unwrap(), second_row.bytes);
    assert_ne!(change_version(&first), first_version);
    assert_ne!(change_version(&second), second_version);
}

#[test]
fn target_change_version_rejects_same_inode_mutate_restore_aba() {
    let fixture = Fixture::new("target-same-inode-mutate-restore-aba");
    fixture.install_all();
    let row = CANONICAL_TEMPLATES
        .iter()
        .find(|row| row.target_path == "AGENTS.md")
        .unwrap();
    let path = fixture.root.join(row.target_path);
    let inode = fs::metadata(&path).unwrap().ino();
    let version = change_version(&path);
    let context = fixture.context();
    let request = fixture.request(&context);
    let attack_path = path.clone();
    let other = same_length_other(row.bytes);
    target_capture_hook_for_test(TargetCapturePhase::BeforeFinalChainRecheck, "", move || {
        fs::write(&attack_path, other).unwrap();
        fs::write(attack_path, row.bytes).unwrap();
    });
    let result = new_authority().issue(
        &context,
        &request,
        fixture.effects(&request),
        10,
        20,
        &nonce("target-same-inode-mutate-restore-aba"),
    );
    assert_target_capture_hook_consumed_for_test();
    let failure = match result {
        Err(failure) => failure,
        Ok(_) => panic!("a target mutate-restore ABA unexpectedly issued authority"),
    };
    assert_eq!(failure.id(), AdapterErrorId::TargetUnavailable);
    assert_late_regular_write_remains(&path, inode, row.bytes, version);
}

#[test]
fn target_complete_collect_rejects_content_length_mode_ownership_link_special_and_path_races() {
    assert_target_complete_collect_race("target-content-race", |target, _, _| {
        let bytes = fs::read(target).unwrap();
        fs::write(target, same_length_other(&bytes)).unwrap();
    });
    assert_target_complete_collect_race("target-length-race", |target, _, _| {
        let mut bytes = fs::read(target).unwrap();
        bytes.push(b'x');
        fs::write(target, bytes).unwrap();
    });
    assert_target_complete_collect_race("target-mode-race", |target, _, _| {
        fs::set_permissions(target, fs::Permissions::from_mode(0o600)).unwrap();
    });
    assert_target_complete_collect_race("target-ownership-race", |target, _, _| {
        let current = fs::metadata(target).unwrap().gid();
        let alternate = alternate_supplementary_gid(current);
        let encoded = std::ffi::CString::new(target.as_os_str().as_encoded_bytes()).unwrap();
        assert_eq!(
            unsafe {
                libc::chown(
                    encoded.as_ptr(),
                    !0 as libc::uid_t,
                    alternate as libc::gid_t,
                )
            },
            0
        );
        assert_eq!(fs::metadata(target).unwrap().gid(), alternate);
    });
    assert_target_complete_collect_race("target-link-race", |target, root, _| {
        fs::hard_link(target, root.join("target-hardlink-race-alias")).unwrap();
    });
    assert_target_complete_collect_race("target-special-race", |target, _, container| {
        fs::rename(target, container.join("target-special-race-original")).unwrap();
        let encoded = std::ffi::CString::new(target.as_os_str().as_encoded_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(encoded.as_ptr(), 0o600) }, 0);
    });
    assert_target_complete_collect_race("target-path-race", |target, _, _| {
        fs::rename(target, target.with_file_name("Agents.md")).unwrap();
    });
}

#[test]
fn mutation_after_postflight_cannot_cross_the_final_green_ancestor_boundary() {
    assert_final_green_ancestor_drift(
        "unique-ancestor-replacement-final-green",
        "validation_artifacts/harness",
        |path, displaced| replace_directory_preserving_children(&path, &displaced),
    );
    assert_final_green_ancestor_drift(
        "shared-ancestor-replacement-final-green",
        "agent-standards",
        |path, displaced| replace_directory_preserving_children(&path, &displaced),
    );
    assert_final_green_ancestor_drift("ancestor-mode-final-green", ".harness", |path, _| {
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
    });
    assert_final_green_ancestor_drift(
        "ancestor-symlink-final-green",
        ".codex/environments",
        |path, displaced| {
            fs::rename(&path, &displaced).unwrap();
            symlink(&displaced, path).unwrap();
        },
    );
    assert_final_green_ancestor_drift(
        "ancestor-special-final-green",
        "validation_artifacts/harness",
        |path, displaced| replace_directory_with_fifo(&path, &displaced),
    );
    assert_final_green_ancestor_drift(
        "ancestor-case-alias-final-green",
        "agent-standards",
        |path, _| {
            let alias = path.parent().unwrap().join("Agent-Standards");
            fs::rename(path, alias).unwrap();
        },
    );
}

#[test]
fn within_postflight_and_final_capture_swaps_are_terminally_ambiguous() {
    for phase in ["postflight", "final"] {
        let fixture = Fixture::new(&format!("within-{phase}-capture-swap"));
        let context = fixture.context();
        let request = fixture.request(&context);
        let authority = new_authority();
        let (permit, lease) = authority
            .issue(
                &context,
                &request,
                fixture.effects(&request),
                10,
                20,
                &nonce(&format!("within-{phase}-capture-swap")),
            )
            .unwrap();
        let active = fixture.root.join("agent-standards");
        let displaced = fixture.container.join(format!("within-{phase}-capture-a"));
        let attacker = Arc::new(Mutex::new(None::<JoinHandle<()>>));
        let attacker_slot = Arc::clone(&attacker);
        let arm = move || {
            let handle = arm_capture_barrier(
                TargetCapturePhase::AfterParentHeldBeforeDescend,
                "agent-standards",
                move || replace_directory_preserving_children(&active, &displaced),
            );
            *attacker_slot.lock().unwrap() = Some(handle);
        };
        if phase == "postflight" {
            before_postflight_observation_for_test(arm);
        } else {
            before_final_green_observation_for_test(arm);
        }
        let failure = expect_apply_failure(apply_with_root_permit(
            &context,
            request,
            Some(permit),
            Some(lease),
            10,
        ));
        attacker.lock().unwrap().take().unwrap().join().unwrap();
        assert_target_capture_hook_consumed_for_test();
        assert_eq!(failure.error().id(), AdapterErrorId::ApplyOutcomeAmbiguous);
        assert!(failure.effect_started());
        assert!(!failure.rollback_complete());
    }
}

#[test]
fn linked_special_and_case_aliased_targets_refuse_before_effect() {
    for kind in ["symlink", "hardlink", "fifo", "socket"] {
        let fixture = Fixture::new(&format!("special-{kind}"));
        let context = fixture.context();
        let request = fixture.request(&context);
        let authority = new_authority();
        let (permit, lease) = authority
            .issue(
                &context,
                &request,
                fixture.effects(&request),
                10,
                20,
                &nonce(kind),
            )
            .unwrap();
        let target = fixture.root.join("AGENTS.md");
        let mut socket = None;
        match kind {
            "symlink" => symlink("elsewhere", &target).unwrap(),
            "hardlink" => {
                fixture.write("hardlink-source", b"linked");
                fs::hard_link(fixture.root.join("hardlink-source"), &target).unwrap();
            }
            "fifo" => {
                let path = std::ffi::CString::new(target.as_os_str().as_encoded_bytes()).unwrap();
                assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
            }
            "socket" => socket = Some(std::os::unix::net::UnixListener::bind(&target).unwrap()),
            _ => unreachable!(),
        }
        let failure = expect_apply_failure(apply_with_root_permit(
            &context,
            request,
            Some(permit),
            Some(lease),
            10,
        ));
        assert!(!failure.effect_started(), "{kind}");
        drop(socket);
    }

    let alias = Fixture::new("case-alias-target");
    alias.write("agents.md", b"alias");
    let context = alias.context();
    let failure = plan_target(&context).unwrap_err();
    assert_eq!(failure.id(), AdapterErrorId::TargetUnavailable);
    assert!(CanonicalPath::parse("../AGENTS.md").is_err());
}

#[test]
fn conflict_and_unowned_overwrite_never_reach_permit_issuance() {
    let fixture = Fixture::new("unowned-conflict");
    fixture.write("AGENTS.md", b"user-owned bytes\n");
    let context = fixture.context();
    let record = plan_target(&context).unwrap();
    assert_eq!(record.conflict_count(), 1);
    let failure = match prepare_apply_request(
        &context,
        &record.to_machine_bytes().unwrap(),
        record.plan_sha256(),
    ) {
        Err(failure) => failure,
        Ok(_) => panic!("conflicting plan unexpectedly prepared an apply request"),
    };
    assert_eq!(failure.id(), AdapterErrorId::PlanConflict);
}

#[test]
fn protected_unowned_state_drift_and_out_of_plan_effects_refuse_without_mutation() {
    let fixture = Fixture::new("protected-state-drift");
    fixture.write("private/unowned.txt", b"original protected bytes\n");
    let context = fixture.context();
    let request = fixture.request(&context);
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            fixture.effects(&request),
            10,
            20,
            &nonce("protected-state-drift"),
        )
        .unwrap();
    fixture.write("private/unowned.txt", b"substituted protected bytes\n");
    let before = snapshot(&fixture.root);
    let failure = expect_apply_failure(apply_with_root_permit(
        &context,
        request,
        Some(permit),
        Some(lease),
        10,
    ));
    assert!(!failure.effect_started());
    assert_eq!(snapshot(&fixture.root), before);

    let fixture = Fixture::new("out-of-plan-scope");
    let context = fixture.context();
    let request = fixture.request(&context);
    let before = snapshot(&fixture.root);
    let (error, scope_violation) =
        scope_violation_for_test(&fixture.root, &request, fixture.effects(&request));
    assert_eq!(error, FitErrorId::Unauthorized);
    assert!(scope_violation);
    assert_eq!(snapshot(&fixture.root), before);
}

#[test]
fn protected_descendants_inside_managed_ancestors_are_preserved_and_bound() {
    let fixture = Fixture::new("managed-ancestor-protected-descendant");
    fixture.write(
        "agent-standards/unrelated.txt",
        b"preserve this protected descendant\n",
    );
    let context = fixture.context();
    let request = fixture.request(&context);
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            fixture.effects(&request),
            10,
            20,
            &nonce("managed-ancestor-protected-descendant"),
        )
        .unwrap();
    let outcome = expect_apply_ok(apply_with_root_permit(
        &context,
        request,
        Some(permit),
        Some(lease),
        10,
    ));
    assert_eq!(outcome.status(), "applied");
    assert_eq!(
        fs::read(fixture.root.join("agent-standards/unrelated.txt")).unwrap(),
        b"preserve this protected descendant\n"
    );

    let drift = Fixture::new("managed-ancestor-protected-drift");
    drift.write("agent-standards/unrelated.txt", b"protected before\n");
    let context = drift.context();
    let request = drift.request(&context);
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            drift.effects(&request),
            10,
            20,
            &nonce("managed-ancestor-protected-drift"),
        )
        .unwrap();
    drift.write("agent-standards/unrelated.txt", b"protected after\n");
    let failure = expect_apply_failure(apply_with_root_permit(
        &context,
        request,
        Some(permit),
        Some(lease),
        10,
    ));
    assert!(!failure.effect_started());
}

#[test]
fn protected_links_and_special_objects_fail_closed_before_effects() {
    for kind in ["symlink", "hardlink", "fifo", "socket"] {
        let fixture = Fixture::new(&format!("protected-{kind}"));
        fixture.write("private/source", b"protected source\n");
        let candidate = fixture.root.join("private/candidate");
        let mut socket = None;
        match kind {
            "symlink" => symlink(fixture.root.join("private/source"), &candidate).unwrap(),
            "hardlink" => fs::hard_link(fixture.root.join("private/source"), &candidate).unwrap(),
            "fifo" => {
                let path =
                    std::ffi::CString::new(candidate.as_os_str().as_encoded_bytes()).unwrap();
                assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
            }
            "socket" => socket = Some(std::os::unix::net::UnixListener::bind(&candidate).unwrap()),
            _ => unreachable!(),
        }
        let context = fixture.context();
        let request = fixture.request(&context);
        let result = new_authority().issue(
            &context,
            &request,
            fixture.effects(&request),
            10,
            20,
            &nonce(&format!("protected-{kind}")),
        );
        let failure = match result {
            Err(failure) => failure,
            Ok(_) => panic!("a protected {kind} unexpectedly issued authority"),
        };
        assert_eq!(failure.id(), AdapterErrorId::TargetUnavailable, "{kind}");
        assert!(
            CANONICAL_TEMPLATES
                .iter()
                .all(|row| !fixture.root.join(row.target_path).is_file()),
            "{kind}"
        );
        drop(socket);
    }
}

#[test]
fn protected_late_same_inode_write_is_detected_and_left_exactly_observable() {
    let fixture = Fixture::new("protected-late-same-inode-write-issuance");
    fixture.write("private/one.txt", b"protected-alpha\n");
    let path = fixture.root.join("private/one.txt");
    let metadata = fs::metadata(&path).unwrap();
    let inode = metadata.ino();
    let version = change_version(&path);
    let context = fixture.context();
    let request = fixture.request(&context);
    let attack_path = path.clone();
    let attacker = arm_protected_capture_barrier(
        ProtectedCaptureBoundary::PermitIssuance,
        ProtectedCapturePhase::AfterRegularRowRevalidated,
        b"private/one.txt",
        move || fs::write(attack_path, b"protected-bravo\n").unwrap(),
    );
    let result = new_authority().issue(
        &context,
        &request,
        fixture.effects(&request),
        10,
        20,
        &nonce("protected-late-same-inode-write-issuance"),
    );
    assert_protected_capture_hook_consumed_for_test();
    attacker.join().unwrap();
    let failure = match result {
        Err(failure) => failure,
        Ok(_) => panic!("a late same-inode protected write issued authority"),
    };
    assert_eq!(failure.id(), AdapterErrorId::TargetUnavailable);
    assert_late_regular_write_remains(&path, inode, b"protected-bravo\n", version);
    assert!(
        CANONICAL_TEMPLATES
            .iter()
            .all(|row| !fixture.root.join(row.target_path).is_file())
    );
}

#[test]
fn protected_two_file_hybrid_collect_is_rejected_without_path_sampling() {
    let fixture = Fixture::new("protected-two-file-hybrid-issuance");
    fixture.write("private/a.txt", b"file-a-before\n");
    fixture.write("private/b.txt", b"file-b-before\n");
    let first = fixture.root.join("private/a.txt");
    let second = fixture.root.join("private/b.txt");
    let first_inode = fs::metadata(&first).unwrap().ino();
    let second_inode = fs::metadata(&second).unwrap().ino();
    let context = fixture.context();
    let request = fixture.request(&context);
    let attack_first = first.clone();
    let attack_second = second.clone();
    let attacker = arm_protected_capture_barrier(
        ProtectedCaptureBoundary::PermitIssuance,
        ProtectedCapturePhase::AfterRegularRowRevalidated,
        b"private/a.txt",
        move || {
            fs::write(attack_first, b"file-a-after!\n").unwrap();
            fs::write(attack_second, b"file-b-after!\n").unwrap();
        },
    );
    let result = new_authority().issue(
        &context,
        &request,
        fixture.effects(&request),
        10,
        20,
        &nonce("protected-two-file-hybrid-issuance"),
    );
    assert_protected_capture_hook_consumed_for_test();
    attacker.join().unwrap();
    let failure = match result {
        Err(failure) => failure,
        Ok(_) => panic!("a coordinated protected hybrid issued authority"),
    };
    assert_eq!(failure.id(), AdapterErrorId::TargetUnavailable);
    assert_eq!(fs::metadata(&first).unwrap().ino(), first_inode);
    assert_eq!(fs::metadata(&second).unwrap().ino(), second_inode);
    assert_eq!(fs::read(&first).unwrap(), b"file-a-after!\n");
    assert_eq!(fs::read(&second).unwrap(), b"file-b-after!\n");
}

#[test]
fn protected_change_version_rejects_same_inode_mutate_restore_aba() {
    let fixture = Fixture::new("protected-mutate-restore-aba");
    fixture.write("private/aba.txt", b"aba-original\n");
    let path = fixture.root.join("private/aba.txt");
    let inode = fs::metadata(&path).unwrap().ino();
    let version = change_version(&path);
    let context = fixture.context();
    let request = fixture.request(&context);
    let attack_path = path.clone();
    let attacker = arm_protected_capture_barrier(
        ProtectedCaptureBoundary::PermitIssuance,
        ProtectedCapturePhase::BeforeFinalRecheck,
        b"",
        move || {
            fs::write(&attack_path, b"aba-mutated!\n").unwrap();
            fs::write(attack_path, b"aba-original\n").unwrap();
        },
    );
    let result = new_authority().issue(
        &context,
        &request,
        fixture.effects(&request),
        10,
        20,
        &nonce("protected-mutate-restore-aba"),
    );
    assert_protected_capture_hook_consumed_for_test();
    attacker.join().unwrap();
    let failure = match result {
        Err(failure) => failure,
        Ok(_) => panic!("a protected mutate-restore ABA issued authority"),
    };
    assert_eq!(failure.id(), AdapterErrorId::TargetUnavailable);
    assert_late_regular_write_remains(&path, inode, b"aba-original\n", version);
}

#[test]
fn protected_descriptor_capture_rejects_permit_issuance_ab_swap() {
    let fixture = Fixture::new("protected-permit-issuance-ab-swap");
    let (active, replacement, displaced) = protected_directory_swap(&fixture, "permit-issuance");
    let context = fixture.context();
    let request = fixture.request(&context);
    let attacker = arm_protected_capture_barrier(
        ProtectedCaptureBoundary::PermitIssuance,
        ProtectedCapturePhase::AfterEnumerationBeforeChildOpen,
        b"private",
        move || swap_protected_directories(active, replacement, displaced),
    );
    let result = new_authority().issue(
        &context,
        &request,
        fixture.effects(&request),
        10,
        20,
        &nonce("protected-permit-issuance-ab-swap"),
    );
    assert_protected_capture_hook_consumed_for_test();
    attacker.join().unwrap();
    let failure = match result {
        Err(failure) => failure,
        Ok(_) => panic!("a protected A/B issuance hybrid unexpectedly issued authority"),
    };
    assert_eq!(failure.id(), AdapterErrorId::TargetUnavailable);
    assert!(
        CANONICAL_TEMPLATES
            .iter()
            .all(|row| !fixture.root.join(row.target_path).is_file())
    );
}

#[test]
fn protected_descriptor_capture_rejects_immediate_pre_effect_ab_swap() {
    let fixture = Fixture::new("protected-pre-effect-ab-swap");
    let (active, replacement, displaced) =
        protected_directory_swap(&fixture, "immediate-pre-effect");
    let context = fixture.context();
    let request = fixture.request(&context);
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            fixture.effects(&request),
            10,
            20,
            &nonce("protected-pre-effect-ab-swap"),
        )
        .unwrap();
    let attacker = arm_protected_capture_barrier(
        ProtectedCaptureBoundary::ImmediatePreEffect,
        ProtectedCapturePhase::AfterDirectoryHeldBeforeDescend,
        b"private",
        move || swap_protected_directories(active, replacement, displaced),
    );
    let result = apply_with_root_permit(&context, request, Some(permit), Some(lease), 10);
    assert_protected_capture_hook_consumed_for_test();
    attacker.join().unwrap();
    let failure = expect_apply_failure(result);
    assert_eq!(failure.error().id(), AdapterErrorId::TargetUnavailable);
    assert!(!failure.effect_started());
}

#[test]
fn protected_descriptor_capture_cannot_hide_postflight_undeclared_write() {
    let fixture = Fixture::new("protected-postflight-hidden-write");
    let (active, replacement, displaced) =
        protected_directory_swap(&fixture, "postflight-hidden-write");
    let displaced_probe = displaced.clone();
    let context = fixture.context();
    let request = fixture.request(&context);
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            UndeclaredWrite {
                inner: fixture.effects(&request),
                root: fixture.root.clone(),
                fired: false,
            },
            10,
            20,
            &nonce("protected-postflight-hidden-write"),
        )
        .unwrap();
    let attacker = arm_protected_capture_barrier(
        ProtectedCaptureBoundary::Postflight,
        ProtectedCapturePhase::AfterEnumerationBeforeChildOpen,
        b"private",
        move || swap_protected_directories(active, replacement, displaced),
    );
    let result = apply_with_root_permit(&context, request, Some(permit), Some(lease), 10);
    assert_protected_capture_hook_consumed_for_test();
    attacker.join().unwrap();
    let failure = expect_apply_failure(result);
    assert_eq!(failure.error().id(), AdapterErrorId::ApplyOutcomeAmbiguous);
    assert!(failure.effect_started());
    assert!(!failure.rollback_complete());
    assert!(displaced_probe.join("undeclared-private-canary").is_file());
    assert!(
        !failure
            .error()
            .to_string()
            .contains("do-not-echo-this-private-canary")
    );
}

#[test]
fn protected_descriptor_capture_rejects_final_green_ab_swap() {
    let fixture = Fixture::new("protected-final-green-ab-swap");
    let (active, replacement, displaced) = protected_directory_swap(&fixture, "final-green");
    let context = fixture.context();
    let request = fixture.request(&context);
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            fixture.effects(&request),
            10,
            20,
            &nonce("protected-final-green-ab-swap"),
        )
        .unwrap();
    let attacker = arm_protected_capture_barrier(
        ProtectedCaptureBoundary::FinalGreen,
        ProtectedCapturePhase::AfterDirectoryHeldBeforeDescend,
        b"private",
        move || swap_protected_directories(active, replacement, displaced),
    );
    let result = apply_with_root_permit(&context, request, Some(permit), Some(lease), 10);
    assert_protected_capture_hook_consumed_for_test();
    attacker.join().unwrap();
    let failure = expect_apply_failure(result);
    assert_eq!(failure.error().id(), AdapterErrorId::ApplyOutcomeAmbiguous);
    assert!(failure.effect_started());
    assert!(!failure.rollback_complete());
}

#[test]
fn protected_descriptor_capture_rejects_rollback_ab_swap() {
    let fixture = Fixture::new("protected-rollback-ab-swap");
    let (active, replacement, displaced) = protected_directory_swap(&fixture, "rollback");
    let context = fixture.context();
    let request = fixture.request(&context);
    let mut effects = fixture.effects(&request);
    effects.fail_on_calls([2]);
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            effects,
            10,
            20,
            &nonce("protected-rollback-ab-swap"),
        )
        .unwrap();
    let attacker = arm_protected_capture_barrier(
        ProtectedCaptureBoundary::Rollback,
        ProtectedCapturePhase::AfterEnumerationBeforeChildOpen,
        b"private",
        move || swap_protected_directories(active, replacement, displaced),
    );
    let result = apply_with_root_permit(&context, request, Some(permit), Some(lease), 10);
    assert_protected_capture_hook_consumed_for_test();
    attacker.join().unwrap();
    let failure = expect_apply_failure(result);
    assert_eq!(failure.error().id(), AdapterErrorId::ApplyOutcomeAmbiguous);
    assert!(failure.effect_started());
    assert!(!failure.rollback_complete());
}

#[test]
fn protected_descriptor_capture_rejects_reconciliation_ab_swap() {
    let fixture = Fixture::new("protected-reconciliation-ab-swap");
    let (active, replacement, displaced) =
        protected_directory_swap(&fixture, "ambiguity-reconciliation");
    let context = fixture.context();
    let request = fixture.request(&context);
    let mut effects = fixture.effects(&request);
    effects.fail_on_calls([2]);
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            effects,
            10,
            20,
            &nonce("protected-reconciliation-ab-swap"),
        )
        .unwrap();
    let attacker = arm_protected_capture_barrier(
        ProtectedCaptureBoundary::AmbiguityReconciliation,
        ProtectedCapturePhase::AfterDirectoryHeldBeforeDescend,
        b"private",
        move || swap_protected_directories(active, replacement, displaced),
    );
    let result = apply_with_root_permit(&context, request, Some(permit), Some(lease), 10);
    assert_protected_capture_hook_consumed_for_test();
    attacker.join().unwrap();
    let failure = expect_apply_failure(result);
    assert_eq!(failure.error().id(), AdapterErrorId::ApplyOutcomeAmbiguous);
    assert!(failure.effect_started());
    assert!(!failure.rollback_complete());
}

#[test]
fn protected_pre_effect_late_length_change_refuses_without_effect() {
    let fixture = Fixture::new("protected-pre-effect-late-length-write");
    fixture.write("private/length.txt", b"short\n");
    let path = fixture.root.join("private/length.txt");
    let inode = fs::metadata(&path).unwrap().ino();
    let version = change_version(&path);
    let context = fixture.context();
    let request = fixture.request(&context);
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            fixture.effects(&request),
            10,
            20,
            &nonce("protected-pre-effect-late-length-write"),
        )
        .unwrap();
    let attack_path = path.clone();
    let attacker = arm_protected_capture_barrier(
        ProtectedCaptureBoundary::ImmediatePreEffect,
        ProtectedCapturePhase::AfterRegularRowRevalidated,
        b"private/length.txt",
        move || fs::write(attack_path, b"late length expansion remains\n").unwrap(),
    );
    let result = apply_with_root_permit(&context, request, Some(permit), Some(lease), 10);
    assert_protected_capture_hook_consumed_for_test();
    attacker.join().unwrap();
    let failure = expect_apply_failure(result);
    assert_eq!(failure.error().id(), AdapterErrorId::TargetUnavailable);
    assert!(!failure.effect_started());
    assert_late_regular_write_remains(&path, inode, b"late length expansion remains\n", version);
}

#[test]
fn protected_postflight_late_mode_change_cannot_produce_success_or_rollback() {
    let fixture = Fixture::new("protected-postflight-late-mode-change");
    fixture.write("private/mode.txt", b"mode protected\n");
    let path = fixture.root.join("private/mode.txt");
    let inode = fs::metadata(&path).unwrap().ino();
    let version = change_version(&path);
    let context = fixture.context();
    let request = fixture.request(&context);
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            fixture.effects(&request),
            10,
            20,
            &nonce("protected-postflight-late-mode-change"),
        )
        .unwrap();
    let attack_path = path.clone();
    let attacker = arm_protected_capture_barrier(
        ProtectedCaptureBoundary::Postflight,
        ProtectedCapturePhase::AfterRegularRowRevalidated,
        b"private/mode.txt",
        move || {
            fs::set_permissions(attack_path, fs::Permissions::from_mode(0o600)).unwrap();
        },
    );
    let result = apply_with_root_permit(&context, request, Some(permit), Some(lease), 10);
    assert_protected_capture_hook_consumed_for_test();
    attacker.join().unwrap();
    let failure = expect_apply_failure(result);
    assert_eq!(failure.error().id(), AdapterErrorId::ApplyOutcomeAmbiguous);
    assert!(failure.effect_started());
    assert!(!failure.rollback_complete());
    assert_late_regular_write_remains(&path, inode, b"mode protected\n", version);
    assert_eq!(fs::metadata(path).unwrap().mode() & 0o7777, 0o600);
}

#[test]
fn protected_final_green_target_overlap_is_caught_by_the_after_collect() {
    let fixture = Fixture::new("protected-final-green-recheck-late-write");
    fixture.write("private/final.txt", b"final-before\n");
    let path = fixture.root.join("private/final.txt");
    let inode = fs::metadata(&path).unwrap().ino();
    let version = change_version(&path);
    let context = fixture.context();
    let request = fixture.request(&context);
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            fixture.effects(&request),
            10,
            20,
            &nonce("protected-final-green-recheck-late-write"),
        )
        .unwrap();
    let attack_path = path.clone();
    let attacker = arm_protected_capture_barrier(
        ProtectedCaptureBoundary::FinalGreenRecheck,
        ProtectedCapturePhase::AfterRegularRowRevalidated,
        b"private/final.txt",
        move || fs::write(attack_path, b"final-after!\n").unwrap(),
    );
    let result = apply_with_root_permit(&context, request, Some(permit), Some(lease), 10);
    assert_protected_capture_hook_consumed_for_test();
    attacker.join().unwrap();
    let failure = expect_apply_failure(result);
    assert_eq!(failure.error().id(), AdapterErrorId::ApplyOutcomeAmbiguous);
    assert!(failure.effect_started());
    assert!(!failure.rollback_complete());
    assert_late_regular_write_remains(&path, inode, b"final-after!\n", version);
}

#[test]
fn final_green_rechecks_target_after_protected_after_and_rejects_late_target_aba() {
    let fixture = Fixture::new("target-final-green-post-protected-aba");
    fixture.install_all();
    let row = CANONICAL_TEMPLATES
        .iter()
        .find(|row| row.target_path == "AGENTS.md")
        .unwrap();
    let path = fixture.root.join(row.target_path);
    let inode = fs::metadata(&path).unwrap().ino();
    let version = change_version(&path);
    let context = fixture.context();
    let request = fixture.request(&context);
    assert!(request.plan.mutations.is_empty());
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            fixture.effects(&request),
            10,
            20,
            &nonce("target-final-green-post-protected-aba"),
        )
        .unwrap();
    let attack_path = path.clone();
    let other = same_length_other(row.bytes);
    let attacker = arm_protected_capture_barrier(
        ProtectedCaptureBoundary::FinalGreenRecheck,
        ProtectedCapturePhase::BeforeFinalRecheck,
        b"",
        move || {
            fs::write(&attack_path, other).unwrap();
            fs::write(attack_path, row.bytes).unwrap();
        },
    );
    let result = apply_with_root_permit(&context, request, Some(permit), Some(lease), 10);
    assert_protected_capture_hook_consumed_for_test();
    attacker.join().unwrap();
    let failure = expect_apply_failure(result);
    assert_eq!(failure.error().id(), AdapterErrorId::ApplyOutcomeAmbiguous);
    assert!(failure.effect_started());
    assert!(!failure.rollback_complete());
    assert_late_regular_write_remains(&path, inode, row.bytes, version);
}

#[test]
fn protected_rollback_late_write_withholds_complete_rollback() {
    let fixture = Fixture::new("protected-rollback-late-write");
    fixture.write("private/rollback.txt", b"rollback-before\n");
    let path = fixture.root.join("private/rollback.txt");
    let inode = fs::metadata(&path).unwrap().ino();
    let version = change_version(&path);
    let context = fixture.context();
    let request = fixture.request(&context);
    let mut effects = fixture.effects(&request);
    effects.fail_on_calls([2]);
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            effects,
            10,
            20,
            &nonce("protected-rollback-late-write"),
        )
        .unwrap();
    let attack_path = path.clone();
    let attacker = arm_protected_capture_barrier(
        ProtectedCaptureBoundary::Rollback,
        ProtectedCapturePhase::AfterRegularRowRevalidated,
        b"private/rollback.txt",
        move || fs::write(attack_path, b"rollback-after-is-left\n").unwrap(),
    );
    let result = apply_with_root_permit(&context, request, Some(permit), Some(lease), 10);
    assert_protected_capture_hook_consumed_for_test();
    attacker.join().unwrap();
    let failure = expect_apply_failure(result);
    assert_eq!(failure.error().id(), AdapterErrorId::ApplyOutcomeAmbiguous);
    assert!(failure.effect_started());
    assert!(!failure.rollback_complete());
    assert_late_regular_write_remains(&path, inode, b"rollback-after-is-left\n", version);
}

#[test]
fn protected_reconciliation_late_write_cannot_fabricate_prior_state() {
    let fixture = Fixture::new("protected-reconciliation-late-write");
    fixture.write("private/reconcile.txt", b"reconcile-before\n");
    let path = fixture.root.join("private/reconcile.txt");
    let inode = fs::metadata(&path).unwrap().ino();
    let version = change_version(&path);
    let context = fixture.context();
    let request = fixture.request(&context);
    let mut effects = fixture.effects(&request);
    effects.fail_on_calls([2]);
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            effects,
            10,
            20,
            &nonce("protected-reconciliation-late-write"),
        )
        .unwrap();
    let attack_path = path.clone();
    let attacker = arm_protected_capture_barrier(
        ProtectedCaptureBoundary::AmbiguityReconciliation,
        ProtectedCapturePhase::AfterRegularRowRevalidated,
        b"private/reconcile.txt",
        move || fs::write(attack_path, b"reconcile-after!\n").unwrap(),
    );
    let result = apply_with_root_permit(&context, request, Some(permit), Some(lease), 10);
    assert_protected_capture_hook_consumed_for_test();
    attacker.join().unwrap();
    let failure = expect_apply_failure(result);
    assert_eq!(failure.error().id(), AdapterErrorId::ApplyOutcomeAmbiguous);
    assert!(failure.effect_started());
    assert!(!failure.rollback_complete());
    assert_late_regular_write_remains(&path, inode, b"reconcile-after!\n", version);
}

#[test]
fn complete_rollback_is_terminal_and_ambiguous_rollback_never_false_passes() {
    let fixture = Fixture::new("complete-rollback");
    fixture.write("private.txt", b"untouched\n");
    let context = fixture.context();
    let request = fixture.request(&context);
    let before = snapshot(&fixture.root);
    let status = git_status(&fixture.root);
    let mut effects = fixture.effects(&request);
    effects.fail_on_calls([2]);
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            effects,
            10,
            20,
            &nonce("complete-rollback"),
        )
        .unwrap();
    let failure = expect_apply_failure(apply_with_root_permit(
        &context,
        request,
        Some(permit),
        Some(lease),
        10,
    ));
    assert_eq!(failure.error().id(), AdapterErrorId::ApplyRolledBack);
    assert!(failure.effect_started());
    assert!(failure.rollback_complete());
    assert_eq!(snapshot(&fixture.root), before);
    assert_eq!(git_status(&fixture.root), status);

    let fixture = Fixture::new("mode-rollback");
    fixture.install_all();
    for row in &CANONICAL_TEMPLATES[..2] {
        fs::set_permissions(
            fixture.root.join(row.target_path),
            fs::Permissions::from_mode(0o600),
        )
        .unwrap();
    }
    let context = fixture.context();
    let request = fixture.request(&context);
    assert_eq!(request.plan.mutations.len(), 2);
    let before = snapshot(&fixture.root);
    let rollback_version = change_version(&fixture.root.join(CANONICAL_TEMPLATES[0].target_path));
    let mut effects = fixture.effects(&request);
    effects.fail_on_calls([2]);
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(&context, &request, effects, 10, 20, &nonce("mode-rollback"))
        .unwrap();
    let failure = expect_apply_failure(apply_with_root_permit(
        &context,
        request,
        Some(permit),
        Some(lease),
        10,
    ));
    assert_eq!(failure.error().id(), AdapterErrorId::ApplyRolledBack);
    assert!(failure.rollback_complete());
    assert_eq!(snapshot(&fixture.root), before);
    assert_ne!(
        change_version(&fixture.root.join(CANONICAL_TEMPLATES[0].target_path)),
        rollback_version,
        "mediator-owned rollback ctime is valid because authorized_target records the post-rollback snapshot"
    );

    let fixture = Fixture::new("ambiguous-rollback");
    let context = fixture.context();
    let request = fixture.request(&context);
    let mut effects = fixture.effects(&request);
    effects.fail_on_calls([2, 3]);
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            effects,
            10,
            20,
            &nonce("ambiguous-rollback"),
        )
        .unwrap();
    let failure = expect_apply_failure(apply_with_root_permit(
        &context,
        request,
        Some(permit),
        Some(lease),
        10,
    ));
    assert_eq!(failure.error().id(), AdapterErrorId::ApplyOutcomeAmbiguous);
    assert!(failure.effect_started());
    assert!(!failure.rollback_complete());
}

struct NoEffect {
    inner: LocalEffects,
}

struct SameInodeAbaThenFail {
    inner: LocalEffects,
    target: PathBuf,
    calls: usize,
}

impl FitReader for SameInodeAbaThenFail {
    fn root_binding(&mut self) -> Result<String, FitError> {
        self.inner.root_binding()
    }

    fn read_file(
        &mut self,
        path: &CanonicalPath,
        maximum_bytes: usize,
    ) -> Result<Option<Vec<u8>>, FitError> {
        self.inner.read_file(path, maximum_bytes)
    }
}

impl FitEffects for SameInodeAbaThenFail {
    fn compare_exchange(
        &mut self,
        path: &CanonicalPath,
        expected: &ExpectedContent,
        replacement: Option<&[u8]>,
    ) -> Result<bool, FitError> {
        self.calls += 1;
        if self.calls == 2 {
            let current = fs::read(&self.target).unwrap();
            fs::write(&self.target, same_length_other(&current)).unwrap();
            fs::write(&self.target, current).unwrap();
            return Err(crate::repository_fit::error(FitErrorId::Unauthorized));
        }
        self.inner.compare_exchange(path, expected, replacement)
    }
}

impl RepositoryFitPermitEffects for SameInodeAbaThenFail {
    fn read_unix_mode(&mut self, path: &CanonicalPath) -> Result<Option<u32>, FitError> {
        self.inner.read_unix_mode(path)
    }
}

#[test]
fn rollback_withholds_attribution_after_unmediated_same_inode_target_aba() {
    let fixture = Fixture::new("target-rollback-attribution-aba");
    let context = fixture.context();
    let request = fixture.request(&context);
    assert!(request.plan.mutations.len() > 1);
    let first_path = fixture.root.join(request.plan.mutations[0].path.as_str());
    let first_desired = request.plan.mutations[0].replacement.clone();
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            SameInodeAbaThenFail {
                inner: fixture.effects(&request),
                target: first_path.clone(),
                calls: 0,
            },
            10,
            20,
            &nonce("target-rollback-attribution-aba"),
        )
        .unwrap();
    let failure = expect_apply_failure(apply_with_root_permit(
        &context,
        request,
        Some(permit),
        Some(lease),
        10,
    ));
    assert_eq!(failure.error().id(), AdapterErrorId::ApplyOutcomeAmbiguous);
    assert!(failure.effect_started());
    assert!(!failure.rollback_complete());
    assert!(first_path.is_file());
    assert_eq!(fs::read(first_path).unwrap(), first_desired);
}

#[test]
fn reconciliation_binds_every_target_collect_to_the_authorized_snapshot() {
    for (label, phase) in [
        (
            "target-reconciliation-before-first-target-aba",
            ReconciliationTargetPhase::AfterAuthorizedRevalidation,
        ),
        (
            "target-reconciliation-between-targets-aba",
            ReconciliationTargetPhase::AfterFirstTarget,
        ),
        (
            "target-reconciliation-after-protected-after-aba",
            ReconciliationTargetPhase::AfterProtectedAfter,
        ),
    ] {
        let fixture = Fixture::new(label);
        fixture.install_all();
        for row in &CANONICAL_TEMPLATES[..2] {
            fs::set_permissions(
                fixture.root.join(row.target_path),
                fs::Permissions::from_mode(0o600),
            )
            .unwrap();
        }
        let context = fixture.context();
        let request = fixture.request(&context);
        assert_eq!(request.plan.mutations.len(), 2, "{label}");
        let path = fixture.root.join(CANONICAL_TEMPLATES[0].target_path);
        let original = fs::read(&path).unwrap();
        let other = same_length_other(&original);
        let mut effects = fixture.effects(&request);
        effects.fail_on_calls([2]);
        let authority = new_authority();
        let (permit, lease) = authority
            .issue(&context, &request, effects, 10, 20, &nonce(label))
            .unwrap();
        let attack_path = path.clone();
        let restored = original.clone();
        let attack_observation = Arc::new(Mutex::new(None));
        let recorded_attack_observation = Arc::clone(&attack_observation);
        reconciliation_target_hook_for_test(phase, move || {
            let inode = fs::metadata(&attack_path).unwrap().ino();
            let version = change_version(&attack_path);
            fs::write(&attack_path, other).unwrap();
            fs::write(&attack_path, restored).unwrap();
            assert_eq!(fs::metadata(&attack_path).unwrap().ino(), inode);
            assert_ne!(change_version(&attack_path), version);
            *recorded_attack_observation.lock().unwrap() = Some(inode);
        });
        let result = apply_with_root_permit(&context, request, Some(permit), Some(lease), 10);
        assert_reconciliation_target_hook_consumed_for_test();
        let failure = expect_apply_failure(result);
        assert_eq!(
            failure.error().id(),
            AdapterErrorId::ApplyOutcomeAmbiguous,
            "{label}"
        );
        assert!(failure.effect_started(), "{label}");
        assert!(!failure.rollback_complete(), "{label}");
        assert_eq!(
            fs::metadata(&path).unwrap().ino(),
            attack_observation.lock().unwrap().unwrap(),
            "{label}"
        );
        assert_eq!(fs::read(&path).unwrap(), original, "{label}");
        assert_eq!(
            fs::metadata(&path).unwrap().mode() & 0o7777,
            0o600,
            "{label}"
        );
    }
}

impl FitReader for NoEffect {
    fn root_binding(&mut self) -> Result<String, FitError> {
        self.inner.root_binding()
    }

    fn read_file(
        &mut self,
        path: &CanonicalPath,
        maximum_bytes: usize,
    ) -> Result<Option<Vec<u8>>, FitError> {
        self.inner.read_file(path, maximum_bytes)
    }
}

impl FitEffects for NoEffect {
    fn compare_exchange(
        &mut self,
        _path: &CanonicalPath,
        _expected: &ExpectedContent,
        _replacement: Option<&[u8]>,
    ) -> Result<bool, FitError> {
        Ok(true)
    }
}

impl RepositoryFitPermitEffects for NoEffect {
    fn read_unix_mode(&mut self, path: &CanonicalPath) -> Result<Option<u32>, FitError> {
        self.inner.read_unix_mode(path)
    }
}

struct RootSwapAfterFirstEffect {
    inner: LocalEffects,
    root: PathBuf,
    displaced: PathBuf,
    fired: bool,
}

impl FitReader for RootSwapAfterFirstEffect {
    fn root_binding(&mut self) -> Result<String, FitError> {
        self.inner.root_binding()
    }

    fn read_file(
        &mut self,
        path: &CanonicalPath,
        maximum_bytes: usize,
    ) -> Result<Option<Vec<u8>>, FitError> {
        self.inner.read_file(path, maximum_bytes)
    }
}

impl FitEffects for RootSwapAfterFirstEffect {
    fn compare_exchange(
        &mut self,
        path: &CanonicalPath,
        expected: &ExpectedContent,
        replacement: Option<&[u8]>,
    ) -> Result<bool, FitError> {
        let changed = self.inner.compare_exchange(path, expected, replacement)?;
        if changed && !self.fired {
            self.fired = true;
            fs::rename(&self.root, &self.displaced).unwrap();
            fs::create_dir(&self.root).unwrap();
        }
        Ok(changed)
    }
}

impl RepositoryFitPermitEffects for RootSwapAfterFirstEffect {
    fn read_unix_mode(&mut self, path: &CanonicalPath) -> Result<Option<u32>, FitError> {
        self.inner.read_unix_mode(path)
    }
}

struct UndeclaredWrite {
    inner: LocalEffects,
    root: PathBuf,
    fired: bool,
}

impl FitReader for UndeclaredWrite {
    fn root_binding(&mut self) -> Result<String, FitError> {
        self.inner.root_binding()
    }

    fn read_file(
        &mut self,
        path: &CanonicalPath,
        maximum_bytes: usize,
    ) -> Result<Option<Vec<u8>>, FitError> {
        self.inner.read_file(path, maximum_bytes)
    }
}

impl FitEffects for UndeclaredWrite {
    fn compare_exchange(
        &mut self,
        path: &CanonicalPath,
        expected: &ExpectedContent,
        replacement: Option<&[u8]>,
    ) -> Result<bool, FitError> {
        if !self.fired {
            self.fired = true;
            fs::create_dir_all(self.root.join("private")).unwrap();
            fs::write(
                self.root.join("private/undeclared-private-canary"),
                b"do-not-echo-this-private-canary",
            )
            .unwrap();
        }
        self.inner.compare_exchange(path, expected, replacement)
    }
}

impl RepositoryFitPermitEffects for UndeclaredWrite {
    fn read_unix_mode(&mut self, path: &CanonicalPath) -> Result<Option<u32>, FitError> {
        self.inner.read_unix_mode(path)
    }
}

#[test]
fn green_receipt_without_effect_and_undeclared_write_cannot_fabricate_success() {
    let fixture = Fixture::new("green-without-effect");
    let context = fixture.context();
    let request = fixture.request(&context);
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            NoEffect {
                inner: fixture.effects(&request),
            },
            10,
            20,
            &nonce("green-without-effect"),
        )
        .unwrap();
    let failure = expect_apply_failure(apply_with_root_permit(
        &context,
        request,
        Some(permit),
        Some(lease),
        10,
    ));
    assert_eq!(failure.error().id(), AdapterErrorId::ApplyRolledBack);
    assert!(failure.rollback_complete());

    let fixture = Fixture::new("undeclared-write");
    let context = fixture.context();
    let request = fixture.request(&context);
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            UndeclaredWrite {
                inner: fixture.effects(&request),
                root: fixture.root.clone(),
                fired: false,
            },
            10,
            20,
            &nonce("undeclared-write"),
        )
        .unwrap();
    let failure = expect_apply_failure(apply_with_root_permit(
        &context,
        request,
        Some(permit),
        Some(lease),
        10,
    ));
    assert_eq!(failure.error().id(), AdapterErrorId::ApplyOutcomeAmbiguous);
    assert!(
        !failure
            .error()
            .to_string()
            .contains("do-not-echo-this-private-canary")
    );
}

#[test]
fn root_replacement_after_the_first_effect_is_terminally_ambiguous() {
    let fixture = Fixture::new("root-swap-after-effect");
    let context = fixture.context();
    let request = fixture.request(&context);
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            RootSwapAfterFirstEffect {
                inner: fixture.effects(&request),
                root: fixture.root.clone(),
                displaced: fixture.container.join("displaced-after-effect"),
                fired: false,
            },
            10,
            20,
            &nonce("root-swap-after-effect"),
        )
        .unwrap();
    let failure = expect_apply_failure(apply_with_root_permit(
        &context,
        request,
        Some(permit),
        Some(lease),
        10,
    ));
    assert_eq!(failure.error().id(), AdapterErrorId::ApplyOutcomeAmbiguous);
    assert!(failure.effect_started());
    assert!(!failure.rollback_complete());
}

#[test]
fn verify_cannot_consume_or_substitute_for_apply_and_secrets_never_echo() {
    let fixture = Fixture::new("verify-as-apply");
    let context = fixture.context();
    let request = fixture.request(&context);
    let canary_secret = b"root-secret-canary-that-must-never-echo-0001";
    let canary_nonce = b"nonce-canary-that-must-never-echo-0000000001";
    let authority = TestRepositoryFitPermitAuthority::new(canary_secret).unwrap();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            fixture.effects(&request),
            10,
            20,
            canary_nonce,
        )
        .unwrap();
    let debug = format!("{permit:?}");
    assert!(!debug.contains(std::str::from_utf8(canary_secret).unwrap()));
    assert!(!debug.contains(std::str::from_utf8(canary_nonce).unwrap()));
    let before = snapshot(&fixture.root);
    let verification = verify_target(&context).unwrap();
    assert!(!verification.idempotent());
    assert_eq!(snapshot(&fixture.root), before);
    assert_eq!(permit_seal_stage_for_test(&request), 0);
    let outcome = expect_apply_ok(apply_with_root_permit(
        &context,
        request,
        Some(permit),
        Some(lease),
        10,
    ));
    assert_eq!(outcome.status(), "applied");
}

#[test]
fn authority_rejects_nonce_reuse_and_duplicate_request_issuance() {
    let fixture = Fixture::new("issuance-replay");
    let context = fixture.context();
    let request = fixture.request(&context);
    let authority = new_authority();
    let (_permit, _lease) = authority
        .issue(
            &context,
            &request,
            fixture.effects(&request),
            10,
            20,
            &nonce("issuance-replay"),
        )
        .unwrap();
    let failure = match authority.issue(
        &context,
        &request,
        fixture.effects(&request),
        10,
        20,
        &nonce("issuance-replay-second-nonce"),
    ) {
        Err(failure) => failure,
        Ok(_) => panic!("duplicate request issuance unexpectedly succeeded"),
    };
    assert_eq!(failure.id(), AdapterErrorId::ApplyPermitReplayed);
}

#[test]
fn authority_rejects_short_secrets_nonces_and_invalid_validity_windows() {
    let failure = match TestRepositoryFitPermitAuthority::new(b"short") {
        Err(failure) => failure,
        Ok(_) => panic!("short root secret unexpectedly created an authority"),
    };
    assert_eq!(failure.id(), AdapterErrorId::ApplyPermitInvalid);

    let fixture = Fixture::new("invalid-authority-inputs");
    let context = fixture.context();
    let request = fixture.request(&context);
    let authority = new_authority();
    let inverted_nonce = nonce("inverted-window");
    let overlong_nonce = nonce("overlong-window");
    for (issued, expires, supplied_nonce) in [
        (10, 20, b"short".as_slice()),
        (20, 19, inverted_nonce.as_slice()),
        (20, 321, overlong_nonce.as_slice()),
    ] {
        let failure = match authority.issue(
            &context,
            &request,
            fixture.effects(&request),
            issued,
            expires,
            supplied_nonce,
        ) {
            Err(failure) => failure,
            Ok(_) => panic!("invalid authority bounds unexpectedly issued a permit"),
        };
        assert_eq!(failure.id(), AdapterErrorId::ApplyPermitInvalid);
    }
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            fixture.effects(&request),
            20,
            30,
            &nonce("valid-after-invalid-inputs"),
        )
        .unwrap();
    let outcome = expect_apply_ok(apply_with_root_permit(
        &context,
        request,
        Some(permit),
        Some(lease),
        20,
    ));
    assert_eq!(outcome.status(), "applied");
}
