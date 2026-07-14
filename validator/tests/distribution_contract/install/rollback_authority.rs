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
    let tx_a = refused_transaction(failure, DistributionErrorId::ProvenanceMismatch);
    assert_eq!(inode_tree(&fixture_a.root), before_a);
    assert_eq!(inode_tree(&fixture_b.root), before_b);
    rollback_install(
        tx_a,
        &mut ScopedInstall::for_scope(fixture_a.confined(), Scope::PersonalFixture),
    )
    .unwrap();
    assert!(!fixture_a.root.join(TARGET).exists());
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
        let _tx = refused_transaction(failure, DistributionErrorId::ObjectChanged);
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
    let _tx = refused_transaction(failure, DistributionErrorId::ObjectUnavailable);
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
    let binding = crate::distribution::JourneyBinding::new(
        package.identity().clone(),
        &host,
        "local-harness-plugins",
    )
    .unwrap();
    tx.bind_journey(&binding).unwrap();
    let before = inode_tree(&fixture.root);

    let failure = rollback_install(
        tx,
        &mut ScopedInstall::for_scope(fixture.confined(), Scope::PersonalFixture),
    )
    .unwrap_err();
    let _tx = refused_transaction(failure, DistributionErrorId::ProvenanceMismatch);
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
    let tx = refused_transaction(failure, DistributionErrorId::ProvenanceMismatch);
    assert_eq!(inode_tree(&fixture.root), before);
    rollback_install(
        tx,
        &mut ScopedInstall::for_scope(fixture.confined(), Scope::PersonalFixture),
    )
    .unwrap();
    assert!(!fixture.root.join(TARGET).exists());
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
    let _tx = refused_transaction(failure, DistributionErrorId::InstallConflict);
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
