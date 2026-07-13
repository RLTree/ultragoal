#![cfg(target_vendor = "apple")]

use super::super::catalog::CANONICAL_TEMPLATES;
use super::super::protocol::OpaqueFitApplyRequest;
use super::super::root_permit::{
    RepositoryFitApplyFailure, RepositoryFitApplyOutcome, RepositoryFitPermitEffects,
    TestRepositoryFitPermitAuthority, apply_with_root_permit,
    before_final_green_observation_for_test, duplicate_authorization_for_test,
    permit_seal_stage_for_test,
};
use super::super::{
    AdapterErrorId, inspect_target, plan_target, prepare_apply_request, verify_target,
};
use super::support::{git_status, snapshot};
use crate::context::{BuildRequest, LiveContext};
use crate::repository_fit::local::LocalEffects;
use crate::repository_fit::{
    CanonicalPath, ExpectedContent, FitEffects, FitError, FitReader, digest,
};
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

const BASE: &str = "/tmp/hul-repository-fit-live-journeys-067";
const TEST_ROOT_SECRET: &[u8] = b"repository-fit-live-journey-test-root-authority-067";
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
        git(
            &root,
            &["config", "user.email", "fit-journey@example.invalid"],
        );
        git(&root, &["config", "user.name", "Repository Fit Journey"]);
        git(&root, &["config", "commit.gpgsign", "false"]);
        git(&root, &["config", "gc.auto", "0"]);
        git(&root, &["config", "maintenance.auto", "false"]);
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
        self.write(row.target_path, row.bytes);
        fs::set_permissions(
            self.root.join(row.target_path),
            fs::Permissions::from_mode(row.unix_mode),
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

    fn commit_all(&self, message: &str) {
        git(&self.root, &["add", "-A"]);
        git(&self.root, &["commit", "--quiet", "-m", message]);
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
        LocalEffects::open_for_test(&self.root, request.unix_modes().clone()).unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        debug_assert!(self.container.starts_with(BASE));
        let _ = fs::remove_dir_all(&self.container);
    }
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

fn authority() -> TestRepositoryFitPermitAuthority {
    TestRepositoryFitPermitAuthority::new(TEST_ROOT_SECRET).unwrap()
}

fn nonce(label: &str) -> Vec<u8> {
    digest(label.as_bytes()).into_bytes()
}

fn issue<E: RepositoryFitPermitEffects>(
    context: &LiveContext,
    request: &OpaqueFitApplyRequest,
    effects: E,
    label: &str,
) -> (
    super::super::root_permit::RepositoryFitApplyPermit,
    super::super::root_permit::RepositoryFitMutationLease<E>,
) {
    authority()
        .issue(context, request, effects, 10, 20, &nonce(label))
        .unwrap()
}

fn apply_ok<E: RepositoryFitPermitEffects>(
    result: Result<RepositoryFitApplyOutcome, RepositoryFitApplyFailure<E>>,
) -> RepositoryFitApplyOutcome {
    match result {
        Ok(outcome) => outcome,
        Err(failure) => panic!("apply failed with {:?}", failure.error().id()),
    }
}

fn apply_failure<E: RepositoryFitPermitEffects>(
    result: Result<RepositoryFitApplyOutcome, RepositoryFitApplyFailure<E>>,
) -> RepositoryFitApplyFailure<E> {
    match result {
        Err(failure) => failure,
        Ok(outcome) => panic!("unexpected {} outcome", outcome.status()),
    }
}

fn apply_once(
    fixture: &Fixture,
    context: &LiveContext,
    request: OpaqueFitApplyRequest,
    label: &str,
) -> RepositoryFitApplyOutcome {
    let (permit, lease) = issue(context, &request, fixture.effects(&request), label);
    apply_ok(apply_with_root_permit(
        context,
        request,
        Some(permit),
        Some(lease),
        10,
    ))
}

fn assert_zero_write<T>(fixture: &Fixture, operation: impl FnOnce() -> T) -> T {
    let before_tree = snapshot(&fixture.root);
    let before_status = git_status(&fixture.root);
    let result = operation();
    assert_eq!(snapshot(&fixture.root), before_tree);
    assert_eq!(git_status(&fixture.root), before_status);
    result
}

#[test]
fn positive_supported_host_fresh_setup_and_repeat_use_are_exact_and_idempotent() {
    let fixture = Fixture::new("positive-fresh-repeat");
    let context = fixture.context();
    let inspection = assert_zero_write(&fixture, || inspect_target(&context).unwrap());
    let record = assert_zero_write(&fixture, || plan_target(&context).unwrap());
    assert_eq!(inspection.classification(), "fresh");
    assert_eq!(record.mutation_count(), CANONICAL_TEMPLATES.len());

    let request = fixture.request(&context);
    let outcome = apply_once(&fixture, &context, request, "positive-fresh");
    assert_eq!(outcome.status(), "applied");
    assert_eq!(outcome.mutation_count(), CANONICAL_TEMPLATES.len());
    for row in CANONICAL_TEMPLATES {
        let path = fixture.root.join(row.target_path);
        assert_eq!(fs::read(&path).unwrap(), row.bytes);
        assert_eq!(
            fs::metadata(path).unwrap().permissions().mode() & 0o7777,
            row.unix_mode
        );
    }

    let repeated_context = fixture.context();
    let repeated_record = assert_zero_write(&fixture, || plan_target(&repeated_context).unwrap());
    assert_eq!(repeated_record.mutation_count(), 0);
    let repeated = fixture.request(&repeated_context);
    let repeated_outcome = apply_once(
        &fixture,
        &repeated_context,
        repeated,
        "positive-repeat-idempotent",
    );
    assert_eq!(repeated_outcome.status(), "idempotent");
    assert_eq!(repeated_outcome.mutation_count(), 0);
    let verification = assert_zero_write(&fixture, || verify_target(&fixture.context()).unwrap());
    assert!(verification.idempotent());
    assert_eq!(verification.matched_files(), CANONICAL_TEMPLATES.len());
}

#[test]
fn positive_supported_host_partial_retrofit_preserves_dirty_user_state() {
    let fixture = Fixture::new("positive-partial-dirty");
    fixture.write("README.md", b"tracked baseline\n");
    fixture.commit_all("baseline");
    fixture.write("README.md", b"tracked dirty user edit\n");
    fixture.write("private/untracked-canary.txt", b"private dirty bytes\n");
    fixture.write_template("AGENTS.md");
    let user_diff = git(&fixture.root, &["diff", "--", "README.md"]).stdout;
    let tracked = fs::read(fixture.root.join("README.md")).unwrap();
    let untracked = fs::read(fixture.root.join("private/untracked-canary.txt")).unwrap();

    let context = fixture.context();
    assert!(context.candidate().dirty);
    let inspection = assert_zero_write(&fixture, || inspect_target(&context).unwrap());
    let record = assert_zero_write(&fixture, || plan_target(&context).unwrap());
    assert_eq!(inspection.classification(), "partial");
    assert_eq!(record.mutation_count(), CANONICAL_TEMPLATES.len() - 1);
    let request = fixture.request(&context);
    let outcome = apply_once(&fixture, &context, request, "positive-partial-dirty");
    assert_eq!(outcome.status(), "applied");
    assert_eq!(fs::read(fixture.root.join("README.md")).unwrap(), tracked);
    assert_eq!(
        fs::read(fixture.root.join("private/untracked-canary.txt")).unwrap(),
        untracked
    );
    assert_eq!(
        git(&fixture.root, &["diff", "--", "README.md"]).stdout,
        user_diff
    );
    assert!(verify_target(&fixture.context()).unwrap().idempotent());
}

#[test]
fn negative_conflict_and_settled_request_replay_refuse_without_effect() {
    let conflict = Fixture::new("negative-conflict");
    conflict.write("AGENTS.md", b"user-owned repository instructions\n");
    conflict.commit_all("user authority");
    let context = conflict.context();
    let before_tree = snapshot(&conflict.root);
    let before_status = git_status(&conflict.root);
    let inspection = inspect_target(&context).unwrap();
    let record = plan_target(&context).unwrap();
    assert_eq!(inspection.classification(), "conflicting");
    assert_eq!(record.conflict_count(), 1);
    let failure = prepare_apply_request(
        &context,
        &record.to_machine_bytes().unwrap(),
        record.plan_sha256(),
    )
    .err()
    .unwrap();
    assert_eq!(failure.id(), AdapterErrorId::PlanConflict);
    assert_eq!(snapshot(&conflict.root), before_tree);
    assert_eq!(git_status(&conflict.root), before_status);

    let replay = Fixture::new("negative-replay");
    replay.install_all_direct();
    let context = replay.context();
    let request = replay.request(&context);
    let duplicate_request = request.duplicate_for_test();
    let (permit, lease) = issue(
        &context,
        &request,
        replay.effects(&request),
        "negative-replay",
    );
    let (duplicate_permit, duplicate_lease) = duplicate_authorization_for_test(
        &permit,
        &duplicate_request,
        replay.effects(&duplicate_request),
    );
    let first = apply_ok(apply_with_root_permit(
        &context,
        request,
        Some(permit),
        Some(lease),
        10,
    ));
    assert_eq!(first.status(), "idempotent");
    let replay_tree = snapshot(&replay.root);
    let replay_status = git_status(&replay.root);
    let failure = apply_failure(apply_with_root_permit(
        &context,
        duplicate_request,
        Some(duplicate_permit),
        Some(duplicate_lease),
        10,
    ));
    assert_eq!(failure.error().id(), AdapterErrorId::ApplyPermitReplayed);
    assert_eq!(snapshot(&replay.root), replay_tree);
    assert_eq!(git_status(&replay.root), replay_status);
}

#[test]
fn mutation_failure_rolls_back_exactly_and_a_new_request_recovers() {
    let fixture = Fixture::new("mutation-rollback-recovery");
    fixture.write("private/user.txt", b"preserve me exactly\n");
    let context = fixture.context();
    let request = fixture.request(&context);
    let before_tree = snapshot(&fixture.root);
    let before_status = git_status(&fixture.root);
    let mut effects = fixture.effects(&request);
    effects.fail_on_calls([2]);
    let (permit, lease) = issue(&context, &request, effects, "mutation-rollback");
    let failure = apply_failure(apply_with_root_permit(
        &context,
        request,
        Some(permit),
        Some(lease),
        10,
    ));
    assert_eq!(failure.error().id(), AdapterErrorId::ApplyRolledBack);
    assert!(failure.effect_started());
    assert!(failure.rollback_complete());
    assert_eq!(snapshot(&fixture.root), before_tree);
    assert_eq!(git_status(&fixture.root), before_status);

    let recovery_context = fixture.context();
    let recovery = fixture.request(&recovery_context);
    let outcome = apply_once(
        &fixture,
        &recovery_context,
        recovery,
        "mutation-rollback-recovery",
    );
    assert_eq!(outcome.status(), "applied");
    assert!(verify_target(&fixture.context()).unwrap().idempotent());
}

#[test]
fn race_after_effect_is_ambiguous_and_fresh_replanning_recovers() {
    let fixture = Fixture::new("race-ambiguous-recovery");
    let context = fixture.context();
    let request = fixture.request(&context);
    let (permit, lease) = issue(
        &context,
        &request,
        fixture.effects(&request),
        "race-ambiguous",
    );
    let victim = fixture.root.join("AGENTS.md");
    before_final_green_observation_for_test(move || fs::remove_file(victim).unwrap());
    let failure = apply_failure(apply_with_root_permit(
        &context,
        request,
        Some(permit),
        Some(lease),
        10,
    ));
    assert_eq!(failure.error().id(), AdapterErrorId::ApplyOutcomeAmbiguous);
    assert!(failure.effect_started());
    assert!(!failure.rollback_complete());

    let recovery_context = fixture.context();
    let inspection = inspect_target(&recovery_context).unwrap();
    let record = plan_target(&recovery_context).unwrap();
    assert_eq!(inspection.classification(), "partial");
    assert_eq!(record.conflict_count(), 0);
    assert_eq!(record.mutation_count(), 1);
    let recovery = fixture.request(&recovery_context);
    let outcome = apply_once(
        &fixture,
        &recovery_context,
        recovery,
        "race-ambiguous-recovery",
    );
    assert_eq!(outcome.status(), "applied");
    assert!(verify_target(&fixture.context()).unwrap().idempotent());
}

#[test]
fn security_link_and_case_alias_substitution_refuse_without_mutation() {
    let symlink_fixture = Fixture::new("security-symlink");
    symlink_fixture.write("private-source", b"private source bytes\n");
    symlink("private-source", symlink_fixture.root.join("AGENTS.md")).unwrap();
    let context = symlink_fixture.context();
    let failure = assert_zero_write(&symlink_fixture, || inspect_target(&context).err().unwrap());
    assert_eq!(failure.id(), AdapterErrorId::TargetUnavailable);

    let hardlink_fixture = Fixture::new("security-hardlink");
    hardlink_fixture.write("private-source", b"private source bytes\n");
    fs::hard_link(
        hardlink_fixture.root.join("private-source"),
        hardlink_fixture.root.join("AGENTS.md"),
    )
    .unwrap();
    let context = hardlink_fixture.context();
    let failure = assert_zero_write(&hardlink_fixture, || {
        inspect_target(&context).err().unwrap()
    });
    assert_eq!(failure.id(), AdapterErrorId::TargetUnavailable);

    let alias_fixture = Fixture::new("security-case-alias");
    alias_fixture.write("Agents.md", b"case alias bytes\n");
    let context = alias_fixture.context();
    let failure = assert_zero_write(&alias_fixture, || inspect_target(&context).err().unwrap());
    assert_eq!(failure.id(), AdapterErrorId::TargetUnavailable);
}

#[test]
fn special_file_fifo_target_refuses_before_effect_without_blocking() {
    let fixture = Fixture::new("special-file-fifo");
    let path = fixture.root.join("AGENTS.md");
    let encoded = std::ffi::CString::new(path.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(encoded.as_ptr(), 0o600) }, 0);
    let context = fixture.context();
    let failure = assert_zero_write(&fixture, || inspect_target(&context).err().unwrap());
    assert_eq!(failure.id(), AdapterErrorId::TargetUnavailable);
}

struct NoEffectSuccess {
    inner: LocalEffects,
}

impl FitReader for NoEffectSuccess {
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

impl FitEffects for NoEffectSuccess {
    fn compare_exchange(
        &mut self,
        _path: &CanonicalPath,
        _expected: &ExpectedContent,
        _replacement: Option<&[u8]>,
    ) -> Result<bool, FitError> {
        Ok(true)
    }
}

impl RepositoryFitPermitEffects for NoEffectSuccess {
    fn read_unix_mode(&mut self, path: &CanonicalPath) -> Result<Option<u32>, FitError> {
        self.inner.read_unix_mode(path)
    }
}

#[test]
fn false_pass_no_effect_success_and_verify_cannot_substitute_for_apply() {
    let fixture = Fixture::new("false-pass-no-effect");
    let context = fixture.context();
    let request = fixture.request(&context);
    let before_tree = snapshot(&fixture.root);
    let before_status = git_status(&fixture.root);
    let verification = verify_target(&context).unwrap();
    assert!(!verification.idempotent());
    assert_eq!(permit_seal_stage_for_test(&request), 0);
    assert_eq!(snapshot(&fixture.root), before_tree);
    assert_eq!(git_status(&fixture.root), before_status);

    let (permit, lease) = issue(
        &context,
        &request,
        NoEffectSuccess {
            inner: fixture.effects(&request),
        },
        "false-pass-no-effect",
    );
    let failure = apply_failure(apply_with_root_permit(
        &context,
        request,
        Some(permit),
        Some(lease),
        10,
    ));
    assert_eq!(failure.error().id(), AdapterErrorId::ApplyRolledBack);
    assert!(failure.effect_started());
    assert!(failure.rollback_complete());
    assert_eq!(snapshot(&fixture.root), before_tree);
    assert_eq!(git_status(&fixture.root), before_status);
}
