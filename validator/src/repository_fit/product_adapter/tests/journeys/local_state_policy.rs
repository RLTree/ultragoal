use super::*;
use crate::repository_fit::product_adapter::{
    plan_target_for_scope, prepare_apply_request,
    root_permit::{
        apply_with_root_permit, before_postflight_observation_for_test, RepositoryFitPermitEffects,
        TestRepositoryFitPermitAuthority,
    },
    AdapterErrorId, FitPlanScope,
};
use crate::repository_fit::{CanonicalPath, ExpectedContent, FitEffects, FitError, FitReader};
use std::process::Stdio;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

#[test]
pub(crate) fn local_state_only_mutation_is_counted_at_each_preparation_surface() {
    let fixture = Fixture::new("local-state-only-count");
    fixture.install_all_direct();
    fs::remove_file(fixture.root.join(".gitignore")).unwrap();
    let context = fixture.context();
    let record = plan_target_for_scope(&context, FitPlanScope::LocalState).unwrap();
    assert_eq!(record.mutation_count(), 1);
    assert!(record.plan.local_state.as_ref().unwrap().mutation_required);
    let prepared = prepare_apply_request(
        &context,
        &record.to_machine_bytes().unwrap(),
        record.plan_sha256(),
    )
    .unwrap();
    assert_eq!(prepared.projection().mutation_count, 1);
    assert_eq!(prepared.request().all_mutations().len(), 1);
    execute(&fixture, prepared).unwrap();
    assert_eq!(
        fs::read(fixture.root.join(".gitignore")).unwrap(),
        b"validation_artifacts/\n"
    );
}

#[test]
pub(crate) fn complete_repository_does_not_create_legacy_target_local_state() {
    let fixture = Fixture::new("complete-repository-no-local-state");
    let context = fixture.context();
    let inspection = assert_zero_write(&fixture, || inspect_target(&context).unwrap());
    assert!(inspection.local_state.is_none());
    let prepared = fixture.plan(&context);
    execute(&fixture, prepared).unwrap();
    assert!(!fixture.root.join(".gitignore").exists());
    let verification = verify_target(&fixture.context()).unwrap();
    assert!(verification.local_state.is_none());
    assert!(verification.idempotent());
}

#[test]
pub(crate) fn local_state_policy_preserves_user_bytes_newline_shape_and_mode() {
    let fixture = Fixture::new("local-state-policy");
    fixture.write(
        ".gitignore",
        b"# repository-owned notes\r\n!validation_artifacts/\r\nprivate/",
    );
    fs::set_permissions(
        fixture.root.join(".gitignore"),
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    fixture.write("unrelated.txt", b"keep me\n");
    let before_status = git_status(&fixture.root);
    let context = fixture.context();
    let record = assert_zero_write(&fixture, || {
        plan_target_for_scope(&context, FitPlanScope::LocalState).unwrap()
    });
    assert_eq!(
        record.local_state.as_ref().unwrap().disposition,
        "needs_update"
    );
    let prepared = prepare_apply_request(
        &context,
        &record.to_machine_bytes().unwrap(),
        record.plan_sha256(),
    )
    .unwrap();
    execute(&fixture, prepared).unwrap();

    assert_eq!(
        fs::read(fixture.root.join(".gitignore")).unwrap(),
        b"# repository-owned notes\r\n!validation_artifacts/\r\nprivate/\r\nvalidation_artifacts/"
    );
    assert_eq!(
        fs::metadata(fixture.root.join(".gitignore"))
            .unwrap()
            .mode()
            & 0o7777,
        0o600
    );
    assert_eq!(
        fs::read(fixture.root.join("unrelated.txt")).unwrap(),
        b"keep me\n"
    );
    assert_eq!(git_status(&fixture.root), before_status);

    let child = fixture
        .root
        .join("validation_artifacts/observability/spool/successor-events-probe.jsonl");
    fixture.write("validation_artifacts/observability/spool/child.txt", b"x");
    git(
        &fixture.root,
        &["check-ignore", "--quiet", child.to_str().unwrap()],
    );
    let tracked = std::process::Command::new("git")
        .args([
            "ls-files",
            "--error-unmatch",
            "--",
            "validation_artifacts/observability/spool/child.txt",
        ])
        .current_dir(&fixture.root)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();
    assert_eq!(tracked.code(), Some(1));
}

#[test]
pub(crate) fn local_state_rollback_restores_crlf_bytes_and_mode_then_fresh_explicit_request_applies(
) {
    let fixture = Fixture::new("local-state-rollback-recovery");
    let original = b"# repository-owned notes\r\n!validation_artifacts/\r\nprivate/";
    let gitignore = fixture.root.join(".gitignore");
    fixture.write(".gitignore", original);
    fs::set_permissions(&gitignore, fs::Permissions::from_mode(0o600)).unwrap();

    let context = fixture.context();
    let record = plan_target_for_scope(&context, FitPlanScope::LocalState).unwrap();
    assert_eq!(record.mutation_count(), 1);
    assert!(record.plan.local_state.as_ref().unwrap().mutation_required);
    let prepared = prepare_apply_request(
        &context,
        &record.to_machine_bytes().unwrap(),
        record.plan_sha256(),
    )
    .unwrap();
    let before_snapshot = snapshot(&fixture.root);
    let before_status = git_status(&fixture.root);
    let fail_postflight_root_binding = Arc::new(AtomicBool::new(false));
    let effects = PostForwardRootBindingFailure {
        inner: effects_for(&fixture, &prepared),
        fail_postflight_root_binding: Arc::clone(&fail_postflight_root_binding),
    };
    let forward_gitignore = gitignore.clone();
    before_postflight_observation_for_test(move || {
        assert_eq!(
            fs::read(&forward_gitignore).unwrap(),
            b"# repository-owned notes\r\n!validation_artifacts/\r\nprivate/\r\nvalidation_artifacts/"
        );
        assert_eq!(
            fs::metadata(&forward_gitignore).unwrap().mode() & 0o7777,
            0o600
        );
        fail_postflight_root_binding.store(true, Ordering::SeqCst);
    });
    let request = prepared.into_request();
    let authority = TestRepositoryFitPermitAuthority::new(
        b"local-state-rollback-recovery-test-root-authority-secret",
    )
    .unwrap();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            effects,
            10,
            20,
            b"local-state-rollback-recovery-failure-nonce",
        )
        .unwrap();
    let failure = match apply_with_root_permit(&context, request, Some(permit), Some(lease), 10) {
        Err(failure) => failure,
        Ok(outcome) => panic!("unexpected {} apply outcome", outcome.status()),
    };
    assert_eq!(failure.error().id(), AdapterErrorId::ApplyRolledBack);
    assert!(failure.effect_started());
    assert!(failure.rollback_complete());
    assert_eq!(fs::read(&gitignore).unwrap(), original);
    assert_eq!(fs::metadata(&gitignore).unwrap().mode() & 0o7777, 0o600);
    assert_eq!(snapshot(&fixture.root), before_snapshot);
    assert_eq!(git_status(&fixture.root), before_status);

    let recovery_context = fixture.context();
    let recovery_record =
        plan_target_for_scope(&recovery_context, FitPlanScope::LocalState).unwrap();
    assert!(
        recovery_record
            .plan
            .local_state
            .as_ref()
            .unwrap()
            .mutation_required
    );
    let recovery = prepare_apply_request(
        &recovery_context,
        &recovery_record.to_machine_bytes().unwrap(),
        recovery_record.plan_sha256(),
    )
    .unwrap();
    let recovery_effects = effects_for(&fixture, &recovery);
    let recovery_request = recovery.into_request();
    let recovery_authority = TestRepositoryFitPermitAuthority::new(
        b"local-state-rollback-recovery-test-root-authority-secret",
    )
    .unwrap();
    let (permit, lease) = recovery_authority
        .issue(
            &recovery_context,
            &recovery_request,
            recovery_effects,
            10,
            20,
            b"local-state-rollback-recovery-success-nonce",
        )
        .unwrap();
    let outcome = match apply_with_root_permit(
        &recovery_context,
        recovery_request,
        Some(permit),
        Some(lease),
        10,
    ) {
        Ok(outcome) => outcome,
        Err(failure) => panic!("recovery failed with {:?}", failure.error().id()),
    };
    assert_eq!(outcome.status(), "applied");
    assert_eq!(outcome.mutation_count(), 1);
    assert_eq!(
        fs::read(&gitignore).unwrap(),
        b"# repository-owned notes\r\n!validation_artifacts/\r\nprivate/\r\nvalidation_artifacts/"
    );
    assert_eq!(fs::metadata(&gitignore).unwrap().mode() & 0o7777, 0o600);
}

struct PostForwardRootBindingFailure {
    inner: crate::repository_fit::local::LocalEffects,
    fail_postflight_root_binding: Arc<AtomicBool>,
}

impl FitReader for PostForwardRootBindingFailure {
    fn root_binding(&mut self) -> Result<String, FitError> {
        if self
            .fail_postflight_root_binding
            .swap(false, Ordering::SeqCst)
        {
            return Err(crate::repository_fit::error(FitErrorId::EffectFailed));
        }
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

impl FitEffects for PostForwardRootBindingFailure {
    fn compare_exchange(
        &mut self,
        path: &CanonicalPath,
        expected: &ExpectedContent,
        replacement: Option<&[u8]>,
    ) -> Result<bool, FitError> {
        self.inner.compare_exchange(path, expected, replacement)
    }
}

impl RepositoryFitPermitEffects for PostForwardRootBindingFailure {}

#[test]
pub(crate) fn already_ignored_local_state_is_a_repeat_noop() {
    let fixture = Fixture::new("local-state-repeat");
    fixture.install_all_direct();
    fixture.write(".gitignore", b"# keep\nvalidation_artifacts/\n");
    let context = fixture.context();
    let record = plan_target_for_scope(&context, FitPlanScope::LocalState).unwrap();
    assert!(!record.plan.local_state.as_ref().unwrap().mutation_required);
    let first = prepare_apply_request(
        &context,
        &record.to_machine_bytes().unwrap(),
        record.plan_sha256(),
    )
    .unwrap();
    let before = snapshot(&fixture.root);
    execute(&fixture, first).unwrap();
    assert_eq!(snapshot(&fixture.root), before);
    let repeated = plan_target_for_scope(&fixture.context(), FitPlanScope::LocalState).unwrap();
    assert!(
        !repeated
            .plan
            .local_state
            .as_ref()
            .unwrap()
            .mutation_required
    );
}
