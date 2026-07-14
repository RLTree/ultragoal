use super::*;

#[test]
pub(crate) fn race_after_effect_is_ambiguous_and_fresh_replanning_recovers() {
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
pub(crate) fn security_link_and_case_alias_substitution_refuse_without_mutation() {
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
pub(crate) fn special_file_fifo_target_refuses_before_effect_without_blocking() {
    let fixture = Fixture::new("special-file-fifo");
    let path = fixture.root.join("AGENTS.md");
    let encoded = std::ffi::CString::new(path.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(encoded.as_ptr(), 0o600) }, 0);
    let context = fixture.context();
    let failure = assert_zero_write(&fixture, || inspect_target(&context).err().unwrap());
    assert_eq!(failure.id(), AdapterErrorId::TargetUnavailable);
}

pub(crate) struct NoEffectSuccess {
    pub(crate) inner: LocalEffects,
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

impl RepositoryFitPermitEffects for NoEffectSuccess {}

#[test]
pub(crate) fn false_pass_no_effect_success_and_verify_cannot_substitute_for_apply() {
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
