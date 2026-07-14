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
