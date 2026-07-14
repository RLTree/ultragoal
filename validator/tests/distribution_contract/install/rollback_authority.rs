#[cfg(unix)]
#[test]
fn confined_rollback_requires_exact_root_and_postimage_authority() {
    let fixture_a = crate::package_journey_fixture::JourneyFixture::new("rollback-root-a");
    let fixture_b = crate::package_journey_fixture::JourneyFixture::new("rollback-root-b");
    let package_a = fixture_a.build("packages/root-a.hugpkg");
    let package_b = fixture_b.build("packages/root-b.hugpkg");
    assert_eq!(package_a.identity(), package_b.identity());
    let tx_a = install_confined(&fixture_a, &package_a, Prior::Absent);
    let _tx_b = install_confined(&fixture_b, &package_b, Prior::Absent);
    let before_a = inode_tree(&fixture_a.root);
    let before_b = inode_tree(&fixture_b.root);

    let failure = rollback_install(
        tx_a,
        &mut ScopedInstall::for_scope(fixture_b.confined(), Scope::PersonalFixture),
    )
    .unwrap_err();
    assert_eq!(failure.id(), DistributionErrorId::ProvenanceMismatch);
    assert_eq!(inode_tree(&fixture_a.root), before_a);
    assert_eq!(inode_tree(&fixture_b.root), before_b);
}

#[cfg(unix)]
#[test]
fn confined_rollback_rejects_same_byte_replacement_and_mutate_restore() {
    for suffix in ["same-byte", "mutate-restore"] {
        let fixture = crate::package_journey_fixture::JourneyFixture::new(suffix);
        let package = fixture.build("packages/package.hugpkg");
        let tx = install_confined(&fixture, &package, Prior::Absent);
        let target = fixture.root.join(TARGET);
        let original = std::fs::read(&target).unwrap();
        if suffix == "mutate-restore" {
            std::fs::write(&target, b"temporary mutation").unwrap();
            std::fs::write(&target, &original).unwrap();
        } else {
            std::fs::remove_file(&target).unwrap();
            std::fs::write(&target, &original).unwrap();
        }
        let attacked = inode_tree(&fixture.root);

        let failure = rollback_install(
            tx,
            &mut ScopedInstall::for_scope(fixture.confined(), Scope::PersonalFixture),
        )
        .unwrap_err();
        assert_eq!(failure.id(), DistributionErrorId::ObjectChanged);
        assert_eq!(inode_tree(&fixture.root), attacked);
    }
}

#[cfg(unix)]
#[test]
fn confined_rollback_rejects_wrong_target_substitution() {
    let fixture = crate::package_journey_fixture::JourneyFixture::new("rollback-target");
    let package = fixture.build("packages/package.hugpkg");
    let tx = install_confined(&fixture, &package, Prior::Absent);
    let target = fixture.root.join(TARGET);
    std::fs::rename(&target, fixture.root.join("installed/elsewhere.hugpkg")).unwrap();
    let moved = inode_tree(&fixture.root);
    let failure = rollback_install(
        tx,
        &mut ScopedInstall::for_scope(fixture.confined(), Scope::PersonalFixture),
    )
    .unwrap_err();
    assert_eq!(failure.id(), DistributionErrorId::ObjectUnavailable);
    assert_eq!(inode_tree(&fixture.root), moved);
}

#[cfg(unix)]
#[test]
fn confined_rollback_rejects_journey_bound_transaction_without_journey_authority() {
    let fixture = crate::package_journey_fixture::JourneyFixture::new("rollback-journey");
    let package = fixture.build("packages/package.hugpkg");
    let mut tx = install_confined_target(
        &fixture,
        &package,
        "plugins/harness-ultragoal.hugpkg",
        Scope::PersonalFixture,
        Prior::Absent,
    );
    let executable = crate::runtime_session::installed_program(&fixture.root);
    let host = crate::distribution::HostCapabilityDeclaration::isolated(
        &fixture.root,
        &fixture.project,
        "isolated-host-v1",
        Some(&executable),
    )
    .unwrap();
    let binding =
        crate::distribution::JourneyBinding::new(
            package.identity().clone(),
            &host,
            "local-harness-plugins",
        )
        .unwrap();
    tx.bind_journey(&binding).unwrap();
    let before = inode_tree(&fixture.root);

    let failure =
        rollback_install(
            tx,
            &mut ScopedInstall::for_scope(fixture.confined(), Scope::PersonalFixture),
        )
        .unwrap_err();
    assert_eq!(failure.id(), DistributionErrorId::ProvenanceMismatch);
    assert_eq!(inode_tree(&fixture.root), before);
}

#[cfg(unix)]
#[test]
fn confined_rollback_rejects_wrong_scope_authority() {
    let fixture = crate::package_journey_fixture::JourneyFixture::new("rollback-scope");
    let package = fixture.build("packages/package.hugpkg");
    let tx = install_confined(&fixture, &package, Prior::Absent);
    let before = inode_tree(&fixture.root);

    let failure = rollback_install(
        tx,
        &mut ScopedInstall::for_scope(fixture.confined(), Scope::RepositoryFixture),
    )
    .unwrap_err();
    assert_eq!(failure.id(), DistributionErrorId::ProvenanceMismatch);
    assert_eq!(inode_tree(&fixture.root), before);
}

#[cfg(unix)]
#[test]
fn confined_rollback_preserves_concurrent_target_replacement() {
    let fixture = crate::package_journey_fixture::JourneyFixture::new("rollback-race");
    let package = fixture.build("packages/package.hugpkg");
    let tx = install_confined(&fixture, &package, Prior::Absent);
    let target = fixture.root.join(TARGET);
    let concurrent = b"concurrent user replacement".to_vec();
    set_test_effect_hook_matching(EffectPoint::Rename, TARGET, {
        let target = target.clone();
        let concurrent = concurrent.clone();
        move |_| {
            std::fs::write(&target, &concurrent).unwrap();
        }
    });

    let failure = rollback_install(
        tx,
        &mut ScopedInstall::for_scope(fixture.confined(), Scope::PersonalFixture),
    )
    .unwrap_err();
    assert_test_effect_hook_consumed();
    assert_eq!(failure.id(), DistributionErrorId::InstallConflict);
    assert_eq!(std::fs::read(target).unwrap(), concurrent);
}

#[cfg(unix)]
#[test]
fn confined_rollback_restores_only_the_original_target() {
    let fixture = crate::package_journey_fixture::JourneyFixture::new("rollback-positive");
    let package = fixture.build("packages/package.hugpkg");
    let prior = b"prior confined install".to_vec();
    std::fs::create_dir_all(fixture.root.join("installed")).unwrap();
    std::fs::write(fixture.root.join(TARGET), &prior).unwrap();
    let tx = install_confined(
        &fixture,
        &package,
        Prior::ExactDigest(crate::distribution_fixture::digest(&prior)),
    );
    rollback_install(tx, &mut ScopedInstall::new(fixture.confined())).unwrap();
    assert_eq!(std::fs::read(fixture.root.join(TARGET)).unwrap(), prior);
}

#[cfg(unix)]
#[test]
fn production_scopes_can_rollback_with_crate_confined_scope_authority() {
    for scope in [Scope::Personal, Scope::Repository] {
        let fixture = crate::package_journey_fixture::JourneyFixture::new("rollback-prod-scope");
        let package = fixture.build("packages/package.hugpkg");
        let target = "plugins/harness-ultragoal.hugpkg";
        let tx = install_confined_target(&fixture, &package, target, scope, Prior::Absent);
        rollback_install(tx, &mut ScopedInstall::for_scope(fixture.confined(), scope)).unwrap();
        assert!(!fixture.root.join(target).exists());
    }
}

#[cfg(unix)]
fn install_confined(
    fixture: &crate::package_journey_fixture::JourneyFixture,
    package: &crate::distribution::PackageSnapshot,
    prior: Prior,
) -> crate::distribution::InstallTransaction {
    install_confined_target(fixture, package, TARGET, Scope::PersonalFixture, prior)
}

#[cfg(unix)]
fn install_confined_target(
    fixture: &crate::package_journey_fixture::JourneyFixture,
    package: &crate::distribution::PackageSnapshot,
    target: &str,
    scope: Scope,
    prior: Prior,
) -> crate::distribution::InstallTransaction {
    let plan = InstallPlan::new(
        package.context_id().into(),
        package.candidate_id().into(),
        scope,
        target.into(),
        package.package_sha256().into(),
        prior,
    )
    .unwrap();
    install(
        &plan,
        package,
        &mut ScopedInstall::for_scope(fixture.confined(), scope),
    )
    .unwrap()
}

#[cfg(unix)]
fn inode_tree(root: &std::path::Path) -> Vec<String> {
    use std::os::unix::fs::MetadataExt;
    let mut rows = Vec::new();
    collect_inode_tree(root, root, &mut rows);
    rows
}

#[cfg(unix)]
fn collect_inode_tree(root: &std::path::Path, path: &std::path::Path, rows: &mut Vec<String>) {
    use std::os::unix::fs::MetadataExt;
    let mut entries = std::fs::read_dir(path)
        .unwrap()
        .map(|entry| entry.unwrap())
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        let meta = std::fs::symlink_metadata(&path).unwrap();
        rows.push(format!(
            "{}:{}:{}:{}:{}",
            path.strip_prefix(root).unwrap().display(),
            meta.dev(),
            meta.ino(),
            meta.len(),
            meta.mode()
        ));
        if meta.is_dir() {
            collect_inode_tree(root, &path, rows);
        }
    }
}
