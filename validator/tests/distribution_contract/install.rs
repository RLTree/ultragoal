use crate::distribution::{
    DistributionErrorId, ExpectedPrior as Prior, InstallEffects, InstallPlan,
    InstallScope as Scope, PackageEffects, build_package, install, plan_package, rollback_install,
    uninstall,
};
use crate::support::{CANDIDATE_ID, CONTEXT_ID, Fixture, digest};
use std::collections::{BTreeMap, BTreeSet};

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
    fn read_installed(&mut self, target: &str, _maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        self.reads += 1;
        if self.fail_read_at == Some(self.reads) {
            return Err(());
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
    ) -> Result<bool, ()> {
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
            return Err(());
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
    fn read_package(&mut self, _maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        Ok(self.0.clone())
    }
    fn compare_exchange_package(
        &mut self,
        expected_sha256: Option<&str>,
        replacement: Option<&[u8]>,
    ) -> Result<bool, ()> {
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
    let observed_digest = transaction.snapshot().package_sha256();
    assert_eq!(observed_digest, package.package_sha256());
    rollback_install(transaction, &mut effects).expect("rollback");
    assert!(!effects.files.contains_key(TARGET));
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
    rollback_install(transaction, &mut effects).expect("restore prior");
    assert_eq!(effects.files[personal], previous);
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
    rollback_install(transaction, &mut effects).expect("idempotent rollback");
    assert_eq!(effects.files[TARGET], package.archive());
    assert_eq!((effects.writes, effects.transitions), (0, 2));
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

#[test]
fn automatic_and_explicit_rollbacks_preserve_concurrent_mutation() {
    let (_fixture, package) = package();
    let plan = install_plan(&package, TARGET, Scope::RepositoryFixture, Prior::Absent);
    let automatic = b"mutation before automatic rollback".to_vec();
    let mut effects = MemoryEffects {
        fail_read_at: Some(2),
        mutate_before_transition: Some((2, ExternalMutation::Replace(automatic.clone()))),
        ..MemoryEffects::default()
    };
    let failure = install(&plan, &package, &mut effects).unwrap_err();
    assert_eq!(failure.id(), DistributionErrorId::InstallConflict);
    assert_eq!(effects.files[TARGET], automatic);
    assert_eq!(effects.writes, 1);
    let explicit = b"mutation before explicit rollback".to_vec();
    let mut effects = MemoryEffects::default();
    let transaction = install(&plan, &package, &mut effects).unwrap();
    effects.mutate_before_transition = Some((2, ExternalMutation::Replace(explicit.clone())));
    let failure = rollback_install(transaction, &mut effects).unwrap_err();
    assert_eq!(failure.id(), DistributionErrorId::InstallConflict);
    assert_eq!(effects.files[TARGET], explicit);
}

#[test]
fn verification_rollback_and_rollback_failure_are_causal() {
    let (_fixture, package) = package();
    let plan = install_plan(&package, TARGET, Scope::RepositoryFixture, Prior::Absent);
    let mut effects = MemoryEffects {
        fail_read_at: Some(2),
        ..MemoryEffects::default()
    };
    let failure = install(&plan, &package, &mut effects).unwrap_err();
    assert_eq!(failure.id(), DistributionErrorId::EffectFailed);
    assert!(!effects.files.contains_key(TARGET));
    let mut effects = MemoryEffects::default();
    let transaction = install(&plan, &package, &mut effects).unwrap();
    effects.fail_transition_at = Some(2);
    let failure = rollback_install(transaction, &mut effects).unwrap_err();
    assert_eq!(failure.id(), DistributionErrorId::RollbackFailed);
    assert_eq!(effects.files[TARGET], package.archive());
}

#[test]
fn special_file_substitution_at_atomic_boundary_is_preserved() {
    let (_fixture, package) = package();
    let target = "installed/SECRET_CANARY.hugpkg";
    let plan = install_plan(&package, target, Scope::RepositoryFixture, Prior::Absent);
    let mut effects = MemoryEffects {
        mutate_before_transition: Some((1, ExternalMutation::Special)),
        ..MemoryEffects::default()
    };
    let failure = install(&plan, &package, &mut effects).unwrap_err();
    assert_eq!(failure.id(), DistributionErrorId::InstallConflict);
    assert!(effects.special_targets.contains(target));
    assert!(!effects.files.contains_key(target));
    assert_eq!(effects.writes, 0);
    assert!(!failure.to_string().contains("SECRET_CANARY"));
}

fn install_plan(
    package: &crate::distribution::PackageSnapshot,
    target: &str,
    scope: Scope,
    expected_prior: Prior,
) -> InstallPlan {
    InstallPlan::new(
        CONTEXT_ID.into(),
        CANDIDATE_ID.into(),
        scope,
        target.into(),
        package.package_sha256().into(),
        expected_prior,
    )
    .unwrap()
}

fn package() -> (Fixture, crate::distribution::PackageSnapshot) {
    let fixture = Fixture::complete("install-package");
    crate::package_manifest::write_sources(&fixture);
    let plan = plan_package(&fixture.root, &crate::package_manifest::spec(false)).unwrap();
    let mut sink = PackageSink(None);
    let package = build_package(&plan, &mut sink).unwrap();
    (fixture, package)
}
