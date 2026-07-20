enum ExternalMutation {
    Replace(Vec<u8>),
    Special,
}

#[derive(Default)]
struct MemoryEffects {
    files: BTreeMap<String, Vec<u8>>,
    special_targets: BTreeSet<String>,
    writes: usize,
    reads: usize,
    transitions: usize,
    mutate_before_transition: Option<(usize, ExternalMutation)>,
    fail_read_at: Option<usize>,
    fail_transition_at: Option<usize>,
}

impl InstallEffects for MemoryEffects {
    fn read_installed(
        &mut self,
        target: &str,
        _maximum: usize,
    ) -> Result<Option<Vec<u8>>, crate::distribution::EffectFailure> {
        self.reads += 1;
        if self.fail_read_at == Some(self.reads) {
            return Err(crate::distribution::EffectFailure);
        }
        if self.special_targets.contains(target) {
            return Ok(None);
        }
        Ok(self.files.get(target).cloned())
    }

    fn compare_exchange_installed(
        &mut self,
        target: &str,
        expected: &Prior,
        replacement: Option<&[u8]>,
    ) -> Result<bool, crate::distribution::EffectFailure> {
        self.transitions += 1;
        if self
            .mutate_before_transition
            .as_ref()
            .is_some_and(|(at, _)| *at == self.transitions)
        {
            let (_, mutation) = self.mutate_before_transition.take().unwrap();
            match mutation {
                ExternalMutation::Replace(bytes) => {
                    self.special_targets.remove(target);
                    self.files.insert(target.to_owned(), bytes);
                }
                ExternalMutation::Special => {
                    self.files.remove(target);
                    self.special_targets.insert(target.to_owned());
                }
            }
        }
        if self.fail_transition_at == Some(self.transitions) {
            return Err(crate::distribution::EffectFailure);
        }
        if self.special_targets.contains(target)
            || !expected_matches(expected, self.files.get(target).map(Vec::as_slice))
        {
            return Ok(false);
        }
        match replacement {
            Some(bytes) if self.files.get(target).map(Vec::as_slice) != Some(bytes) => {
                self.writes += 1;
                self.files.insert(target.to_owned(), bytes.to_vec());
            }
            Some(_) => {}
            None if self.files.remove(target).is_some() => self.writes += 1,
            None => {}
        }
        Ok(true)
    }
}

fn expected_matches(expected: &Prior, actual: Option<&[u8]>) -> bool {
    match expected {
        Prior::Absent => actual.is_none(),
        Prior::ExactDigest(expected) => actual.is_some_and(|bytes| digest(bytes) == *expected),
    }
}

struct PackageSink(Option<Vec<u8>>);

impl PackageEffects for PackageSink {
    fn read_package(
        &mut self,
        _maximum: usize,
    ) -> Result<Option<Vec<u8>>, crate::distribution::EffectFailure> {
        Ok(self.0.clone())
    }
    fn compare_exchange_package(
        &mut self,
        expected_sha256: Option<&str>,
        replacement: Option<&[u8]>,
    ) -> Result<bool, crate::distribution::EffectFailure> {
        let current_sha256 = self.0.as_deref().map(digest);
        if current_sha256.as_deref() != expected_sha256 {
            return Ok(false);
        }
        self.0 = replacement.map(<[u8]>::to_vec);
        Ok(true)
    }
}
const TARGET: &str = "installed/plugin.hugpkg";

#[test]
fn missing_prior_and_idempotent_lifecycles_are_byte_reconciled() {
    let (_fixture, package) = package();
    let plan = install_plan(&package, TARGET, Scope::RepositoryFixture, Prior::Absent);
    let mut effects = MemoryEffects::default();
    let transaction = install(&plan, &package, &mut effects).expect("install");
    let debug = format!("{transaction:?}");
    assert!(!debug.contains(TARGET));
    assert!(!debug.contains("HUGPKG"));
    let snapshot = transaction.snapshot().clone();
    let observed_digest = transaction.snapshot().package_sha256();
    assert_eq!(observed_digest, package.package_sha256());
    assert_eq!(effects.files[TARGET], package.archive());
    uninstall(TARGET, &snapshot, &mut effects).expect("clear non-authoritative install");
    let transaction = install(&plan, &package, &mut effects).expect("reinstall");
    let snapshot = transaction.snapshot().clone();
    uninstall(TARGET, &snapshot, &mut effects).expect("uninstall");
    assert!(!effects.files.contains_key(TARGET));
    let personal = "personal/cache/plugin.hugpkg";
    let previous = b"previous verified package".to_vec();
    let plan = install_plan(
        &package,
        personal,
        Scope::PersonalFixture,
        Prior::ExactDigest(digest(&previous)),
    );
    let mut effects = MemoryEffects::default();
    effects.files.insert(personal.into(), previous.clone());
    let transaction = install(&plan, &package, &mut effects).expect("replacement");
    assert!(transaction.snapshot().replaced_existing());
    assert_eq!(effects.files[personal], package.archive());
    let plan = install_plan(
        &package,
        TARGET,
        Scope::RepositoryFixture,
        Prior::ExactDigest(package.package_sha256().into()),
    );
    effects
        .files
        .insert(TARGET.into(), package.archive().to_vec());
    effects.writes = 0;
    effects.transitions = 0;
    let transaction = install(&plan, &package, &mut effects).expect("idempotent install");
    let snapshot = transaction.snapshot().clone();
    uninstall(TARGET, &snapshot, &mut effects).expect("idempotent uninstall");
    assert!(!effects.files.contains_key(TARGET));
    assert_eq!((effects.writes, effects.transitions), (1, 2));
}

#[test]
fn install_and_uninstall_conflicts_preserve_user_state() {
    let (_fixture, package) = package();
    let plan = install_plan(&package, TARGET, Scope::RepositoryFixture, Prior::Absent);
    let mut effects = MemoryEffects::default();
    effects.files.insert(TARGET.into(), b"user bytes".to_vec());
    let failure = install(&plan, &package, &mut effects).unwrap_err();
    assert_eq!(failure.id(), DistributionErrorId::InstallConflict);
    assert_eq!(effects.writes, 0);
    effects.files.remove(TARGET);
    let transaction = install(&plan, &package, &mut effects).expect("install");
    let snapshot = transaction.snapshot().clone();
    let concurrent = b"substitution".to_vec();
    effects.mutate_before_transition = Some((2, ExternalMutation::Replace(concurrent.clone())));
    let failure = uninstall(TARGET, &snapshot, &mut effects).unwrap_err();
    assert_eq!(failure.id(), DistributionErrorId::InstallConflict);
    assert_eq!(effects.files[TARGET], concurrent);
}

#[test]
fn absent_and_exact_prior_install_races_are_preserved_without_echo() {
    let (_fixture, package) = package();
    for (target, prior) in [
        ("installed/SECRET_ABSENT", None),
        ("installed/SECRET_PRIOR", Some(b"prior".to_vec())),
    ] {
        let expected = prior
            .as_ref()
            .map_or(Prior::Absent, |bytes| Prior::ExactDigest(digest(bytes)));
        let plan = install_plan(&package, target, Scope::RepositoryFixture, expected);
        let concurrent = b"SECRET_CONCURRENT_USER_STATE".to_vec();
        let mut effects = MemoryEffects::default();
        if let Some(prior) = prior {
            effects.files.insert(target.into(), prior);
        }
        effects.mutate_before_transition = Some((1, ExternalMutation::Replace(concurrent.clone())));
        let failure = install(&plan, &package, &mut effects).unwrap_err();
        assert_eq!(failure.id(), DistributionErrorId::InstallConflict);
        assert_eq!(effects.files[target], concurrent);
        assert_eq!((effects.writes, effects.transitions), (0, 1));
        assert!(!failure.to_string().contains("SECRET"));
    }
}
