use super::*;

#[test]
fn crash_between_lineage_and_anchor_recovers_without_reapplying_any_surface() {
    let fixture = Fixture::new("lineage-anchor-crash");
    let package = fixture.bundle("0.0.12");
    let plan = fixture.install_plan(&package);
    assert_eq!(
        fixture
            .adapter
            .execute_with_test_hook(&plan, |point| {
                if point == DarwinTestPoint::AfterLineageBeforeAnchor {
                    DarwinTestControl::Interrupt
                } else {
                    DarwinTestControl::Continue
                }
            })
            .unwrap_err()
            .id(),
        DarwinHostErrorId::Interrupted
    );
    assert!(fixture.lineage_record_path().is_file());
    assert!(fixture.journal_record_path().is_file());
    assert!(!fixture.lineage_anchor_record_path().exists());
    let surfaces =
        DarwinHostSurface::ALL.map(|surface| fs::read(fixture.record_path(surface)).unwrap());
    assert_eq!(
        fixture.adapter.query(&plan).unwrap().diagnosis(),
        DarwinHostDiagnosis::RecoveryPending
    );

    let recovered = reopen(&fixture).unwrap().recover(&plan).unwrap();
    assert_eq!(
        recovered.disposition(),
        DarwinHostTransactionDisposition::Recovered
    );
    assert_eq!(
        recovered.snapshot().diagnosis(),
        DarwinHostDiagnosis::Verified
    );
    assert!(fixture.lineage_anchor_record_path().is_file());
    assert!(!fixture.journal_record_path().exists());
    for (surface, bytes) in DarwinHostSurface::ALL.into_iter().zip(surfaces) {
        assert_eq!(fs::read(fixture.record_path(surface)).unwrap(), bytes);
    }
}

#[test]
fn completed_lineage_survives_a_crash_before_journal_removal_without_reapplying() {
    let fixture = Fixture::new("completed-lineage-crash");
    let package = fixture.bundle("0.0.12");
    let plan = fixture.install_plan(&package);
    assert_eq!(
        fixture
            .adapter
            .execute_with_test_hook(&plan, |point| {
                if point == DarwinTestPoint::BeforeJournalRemoval {
                    DarwinTestControl::Interrupt
                } else {
                    DarwinTestControl::Continue
                }
            })
            .unwrap_err()
            .id(),
        DarwinHostErrorId::Interrupted
    );
    let lineage = fs::read(fixture.lineage_record_path()).unwrap();
    let surfaces =
        DarwinHostSurface::ALL.map(|surface| fs::read(fixture.record_path(surface)).unwrap());
    assert_eq!(
        fixture.adapter.query(&plan).unwrap().diagnosis(),
        DarwinHostDiagnosis::RecoveryPending
    );

    let recovered = reopen(&fixture).unwrap().recover(&plan).unwrap();
    assert_eq!(
        recovered.disposition(),
        DarwinHostTransactionDisposition::Recovered
    );
    assert_eq!(
        recovered.snapshot().diagnosis(),
        DarwinHostDiagnosis::Verified
    );
    assert!(!fixture.journal_record_path().exists());
    assert_eq!(fs::read(fixture.lineage_record_path()).unwrap(), lineage);
    for (surface, bytes) in DarwinHostSurface::ALL.into_iter().zip(surfaces) {
        assert_eq!(fs::read(fixture.record_path(surface)).unwrap(), bytes);
    }
}

#[test]
fn cancelled_lineage_survives_a_crash_before_journal_removal_without_an_effect() {
    let fixture = Fixture::new("cancelled-lineage-crash");
    let package = fixture.bundle("0.0.12");
    let plan = fixture.install_plan(&package);
    fixture
        .adapter
        .execute_with_test_hook(&plan, |point| {
            if point == DarwinTestPoint::BeforeSurface(DarwinHostSurface::Installed) {
                DarwinTestControl::Interrupt
            } else {
                DarwinTestControl::Continue
            }
        })
        .unwrap_err();
    assert_eq!(
        fixture
            .adapter
            .cancel_with_test_hook(&plan, |point| {
                if point == DarwinTestPoint::BeforeCancellationRemoval {
                    DarwinTestControl::Interrupt
                } else {
                    DarwinTestControl::Continue
                }
            })
            .unwrap_err()
            .id(),
        DarwinHostErrorId::Interrupted
    );
    let lineage = fs::read(fixture.lineage_record_path()).unwrap();

    let recovered = reopen(&fixture).unwrap().recover(&plan).unwrap();
    assert_eq!(
        recovered.disposition(),
        DarwinHostTransactionDisposition::Cancelled
    );
    assert_eq!(
        recovered.snapshot().diagnosis(),
        DarwinHostDiagnosis::Absent
    );
    assert_eq!(fs::read(fixture.lineage_record_path()).unwrap(), lineage);
    assert!(!fixture.journal_record_path().exists());
    for surface in DarwinHostSurface::ALL {
        assert!(!fixture.record_path(surface).exists());
    }
}
