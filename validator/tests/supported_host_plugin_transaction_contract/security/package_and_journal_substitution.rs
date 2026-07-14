use super::*;

#[test]
fn package_candidate_version_marketplace_and_archive_substitutions_fail_closed() {
    let candidate = Fixture::new("candidate-substitution");
    let foreign = candidate.bundle_with_candidate("0.0.12", FOREIGN_CANDIDATE);
    let target = candidate.bundle("0.0.12");
    candidate
        .adapter
        .seed_candidate_for_test(&foreign.snapshot)
        .unwrap();
    let plan = candidate.reinstall_plan(&target);
    let snapshot = candidate.adapter.query(&plan).unwrap();
    assert_eq!(snapshot.diagnosis(), DarwinHostDiagnosis::Conflict);
    for surface in DarwinHostSurface::ALL {
        assert_eq!(
            snapshot.surface(surface).status(),
            DarwinSurfaceStatus::Substituted
        );
    }

    let version = Fixture::new("version-substitution");
    let old = version.bundle("0.0.11");
    let target = version.bundle("0.0.12");
    version
        .adapter
        .seed_candidate_for_test(&old.snapshot)
        .unwrap();
    let plan = version
        .adapter
        .plan_update(&target.snapshot, &old.snapshot, MARKETPLACE)
        .unwrap();
    assert_eq!(
        version.adapter.query(&plan).unwrap().runtime().status(),
        DarwinSurfaceStatus::Stale
    );

    let marketplace = installed_fixture("marketplace-substitution");
    let surface = DarwinHostSurface::Marketplace;
    let bytes = fs::read(marketplace.record_path(surface)).unwrap();
    let text = String::from_utf8(bytes)
        .unwrap()
        .replace(MARKETPLACE, "rogue-harness-market");
    marketplace.overwrite_record(surface, text.as_bytes());
    let plan = installed_plan(&marketplace);
    assert_eq!(
        marketplace
            .adapter
            .query(&plan)
            .unwrap()
            .marketplace()
            .status(),
        DarwinSurfaceStatus::Substituted
    );

    let archive = installed_fixture("archive-substitution");
    let record = archive.record_path(DarwinHostSurface::Installed);
    let mut bytes = fs::read(&record).unwrap();
    let last = bytes.last_mut().unwrap();
    *last ^= 1;
    fs::write(&record, bytes).unwrap();
    let plan = installed_plan(&archive);
    assert_eq!(
        archive.adapter.query(&plan).unwrap().installed().status(),
        DarwinSurfaceStatus::Dirty
    );
}

#[test]
fn foreign_journal_and_plan_substitution_are_rejected_without_an_effect() {
    let primary = Fixture::new("journal-substitution-primary");
    let primary_package = primary.bundle("0.0.12");
    let primary_plan = primary.install_plan(&primary_package);
    primary
        .adapter
        .execute_with_test_hook(&primary_plan, |point| {
            if point
                == crate::host_lifecycle::DarwinTestPoint::BeforeSurface(
                    DarwinHostSurface::Installed,
                )
            {
                crate::host_lifecycle::DarwinTestControl::Interrupt
            } else {
                crate::host_lifecycle::DarwinTestControl::Continue
            }
        })
        .unwrap_err();

    let foreign = Fixture::new("journal-substitution-foreign");
    let foreign_package = foreign.bundle("0.0.12");
    let foreign_plan = foreign.install_plan(&foreign_package);
    foreign
        .adapter
        .execute_with_test_hook(&foreign_plan, |point| {
            if point
                == crate::host_lifecycle::DarwinTestPoint::BeforeSurface(
                    DarwinHostSurface::Installed,
                )
            {
                crate::host_lifecycle::DarwinTestControl::Interrupt
            } else {
                crate::host_lifecycle::DarwinTestControl::Continue
            }
        })
        .unwrap_err();
    fs::write(
        primary.journal_record_path(),
        fs::read(foreign.journal_record_path()).unwrap(),
    )
    .unwrap();
    assert_eq!(
        primary.adapter.query(&primary_plan).unwrap_err().id(),
        DarwinHostErrorId::JournalCorrupt
    );
    for surface in DarwinHostSurface::ALL {
        assert!(!primary.record_path(surface).exists());
    }

    let conflict = Fixture::new("journal-plan-conflict");
    let target = conflict.bundle("0.0.12");
    let substituted = conflict.bundle_with_candidate("0.0.12", FOREIGN_CANDIDATE);
    let target_plan = conflict.install_plan(&target);
    let substituted_plan = conflict.install_plan(&substituted);
    conflict
        .adapter
        .execute_with_test_hook(&target_plan, |point| {
            if point
                == crate::host_lifecycle::DarwinTestPoint::BeforeSurface(
                    DarwinHostSurface::Installed,
                )
            {
                crate::host_lifecycle::DarwinTestControl::Interrupt
            } else {
                crate::host_lifecycle::DarwinTestControl::Continue
            }
        })
        .unwrap_err();
    assert_eq!(
        conflict
            .adapter
            .recover(&substituted_plan)
            .unwrap_err()
            .id(),
        DarwinHostErrorId::JournalConflict
    );
    for surface in DarwinHostSurface::ALL {
        assert!(!conflict.record_path(surface).exists());
    }
}

#[test]
fn retained_root_mode_drift_after_adapter_open_refuses_before_host_publication() {
    let fixture = Fixture::new("retained-root-mode-drift");
    let package = fixture.bundle("0.0.12");
    let plan = fixture.install_plan(&package);
    fs::set_permissions(&fixture.root, fs::Permissions::from_mode(0o777)).unwrap();
    let before = fixture.tree();

    assert_eq!(
        fixture.adapter.execute(&plan).unwrap_err().id(),
        DarwinHostErrorId::ObservationChanged
    );
    assert_eq!(fixture.tree(), before);
    assert!(!fixture.journal_record_path().exists());
    assert!(!fixture.lineage_record_path().exists());
    for surface in DarwinHostSurface::ALL {
        assert!(!fixture.record_path(surface).exists());
    }

    fs::set_permissions(&fixture.root, fs::Permissions::from_mode(0o700)).unwrap();
}
