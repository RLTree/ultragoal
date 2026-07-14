use super::*;

#[test]
pub(crate) fn complete_rollback_is_terminal_and_ambiguous_rollback_never_false_passes() {
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

pub(crate) struct NoEffect {
    pub(crate) inner: LocalEffects,
}

pub(crate) struct SameInodeAbaThenFail {
    pub(crate) inner: LocalEffects,
    pub(crate) target: PathBuf,
    pub(crate) calls: usize,
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

impl RepositoryFitPermitEffects for SameInodeAbaThenFail {}

#[test]
pub(crate) fn rollback_withholds_attribution_after_unmediated_same_inode_target_aba() {
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
