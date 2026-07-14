use super::*;

#[test]
pub(crate) fn reconciliation_binds_every_target_collect_to_the_authorized_snapshot() {
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

impl RepositoryFitPermitEffects for NoEffect {}

pub(crate) struct RootSwapAfterFirstEffect {
    pub(crate) inner: LocalEffects,
    pub(crate) root: PathBuf,
    pub(crate) displaced: PathBuf,
    pub(crate) fired: bool,
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

impl RepositoryFitPermitEffects for RootSwapAfterFirstEffect {}

pub(crate) struct UndeclaredWrite {
    pub(crate) inner: LocalEffects,
    pub(crate) root: PathBuf,
    pub(crate) fired: bool,
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
