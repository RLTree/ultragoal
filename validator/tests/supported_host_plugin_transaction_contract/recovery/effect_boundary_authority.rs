use super::*;

#[test]
fn execute_reauthenticates_lineage_and_anchor_at_the_final_surface_boundary() {
    for authority in ["lineage", "anchor"] {
        let fixture = Fixture::new(&format!("execute-effect-authority-{authority}"));
        let package = fixture.bundle("0.0.12");
        let install = fixture.install_plan(&package);
        fixture.adapter.execute(&install).unwrap();
        let stale = match authority {
            "lineage" => fs::read(fixture.lineage_record_path()).unwrap(),
            "anchor" => fs::read(fixture.lineage_anchor_record_path()).unwrap(),
            _ => unreachable!(),
        };
        let uninstall = fixture
            .adapter
            .plan_uninstall(&package.snapshot, MARKETPLACE)
            .unwrap();
        fixture.adapter.execute(&uninstall).unwrap();
        let authority_path = match authority {
            "lineage" => fixture.lineage_record_path(),
            "anchor" => fixture.lineage_anchor_record_path(),
            _ => unreachable!(),
        };
        let canonical = fs::read(&authority_path).unwrap();
        assert_ne!(stale, canonical);
        let plan = fixture.install_plan(&package);
        for surface in DarwinHostSurface::ALL {
            assert!(!fixture.record_path(surface).exists());
        }

        let mut journal_at_boundary = None;
        let error = fixture
            .adapter
            .execute_with_test_hook(&plan, |point| {
                if point == DarwinTestPoint::BeforeCompareExchange(DarwinHostSurface::Installed) {
                    fs::write(&authority_path, &stale).unwrap();
                    journal_at_boundary = Some(fs::read(fixture.journal_record_path()).unwrap());
                }
                DarwinTestControl::Continue
            })
            .unwrap_err();
        assert_eq!(error.id(), DarwinHostErrorId::LineageConflict);
        let claimed_journal = journal_at_boundary.expect("effect-boundary journal");
        assert_eq!(
            fs::read(fixture.journal_record_path()).unwrap(),
            claimed_journal
        );
        assert_eq!(fs::read(&authority_path).unwrap(), stale);
        for surface in DarwinHostSurface::ALL {
            assert!(
                !fixture.record_path(surface).exists(),
                "execute wrote {surface:?} after {authority} authority substitution"
            );
        }

        assert_eq!(
            fixture.adapter.recover(&plan).unwrap_err().id(),
            DarwinHostErrorId::LineageConflict
        );
        assert_eq!(
            fs::read(fixture.journal_record_path()).unwrap(),
            claimed_journal
        );
        for surface in DarwinHostSurface::ALL {
            assert!(!fixture.record_path(surface).exists());
        }

        fs::write(&authority_path, canonical).unwrap();
        let recovered = fixture.adapter.recover(&plan).unwrap();
        assert_eq!(
            recovered.disposition(),
            DarwinHostTransactionDisposition::Recovered
        );
        assert_eq!(
            recovered.snapshot().diagnosis(),
            DarwinHostDiagnosis::Verified
        );
        assert!(!fixture.journal_record_path().exists());
    }
}

#[test]
fn recovery_reauthenticates_lineage_and_anchor_at_the_final_surface_boundary() {
    for authority in ["lineage", "anchor"] {
        let fixture = Fixture::new(&format!("recovery-effect-authority-{authority}"));
        let package = fixture.bundle("0.0.12");
        let install = fixture.install_plan(&package);
        fixture.adapter.execute(&install).unwrap();
        let stale = match authority {
            "lineage" => fs::read(fixture.lineage_record_path()).unwrap(),
            "anchor" => fs::read(fixture.lineage_anchor_record_path()).unwrap(),
            _ => unreachable!(),
        };
        let uninstall = fixture
            .adapter
            .plan_uninstall(&package.snapshot, MARKETPLACE)
            .unwrap();
        fixture.adapter.execute(&uninstall).unwrap();
        let authority_path = match authority {
            "lineage" => fixture.lineage_record_path(),
            "anchor" => fixture.lineage_anchor_record_path(),
            _ => unreachable!(),
        };
        let canonical = fs::read(&authority_path).unwrap();
        assert_ne!(stale, canonical);
        let plan = fixture.install_plan(&package);
        assert_eq!(
            fixture
                .adapter
                .execute_with_test_hook(&plan, |point| {
                    if point == DarwinTestPoint::BeforeSurface(DarwinHostSurface::Installed) {
                        DarwinTestControl::Interrupt
                    } else {
                        DarwinTestControl::Continue
                    }
                })
                .unwrap_err()
                .id(),
            DarwinHostErrorId::Interrupted
        );

        let mut journal_at_boundary = None;
        let error = fixture
            .adapter
            .recover_with_test_hook(&plan, |point| {
                if point == DarwinTestPoint::BeforeCompareExchange(DarwinHostSurface::Installed) {
                    fs::write(&authority_path, &stale).unwrap();
                    journal_at_boundary = Some(fs::read(fixture.journal_record_path()).unwrap());
                }
                DarwinTestControl::Continue
            })
            .unwrap_err();
        assert_eq!(error.id(), DarwinHostErrorId::LineageConflict);
        let claimed_journal = journal_at_boundary.expect("effect-boundary journal");
        assert_eq!(
            fs::read(fixture.journal_record_path()).unwrap(),
            claimed_journal
        );
        assert_eq!(fs::read(&authority_path).unwrap(), stale);
        for surface in DarwinHostSurface::ALL {
            assert!(
                !fixture.record_path(surface).exists(),
                "recovery wrote {surface:?} after {authority} authority substitution"
            );
        }

        assert_eq!(
            fixture.adapter.recover(&plan).unwrap_err().id(),
            DarwinHostErrorId::LineageConflict
        );
        assert_eq!(
            fs::read(fixture.journal_record_path()).unwrap(),
            claimed_journal
        );
        for surface in DarwinHostSurface::ALL {
            assert!(!fixture.record_path(surface).exists());
        }

        fs::write(&authority_path, canonical).unwrap();
        let recovered = fixture.adapter.recover(&plan).unwrap();
        assert_eq!(
            recovered.disposition(),
            DarwinHostTransactionDisposition::Recovered
        );
        assert_eq!(
            recovered.snapshot().diagnosis(),
            DarwinHostDiagnosis::Verified
        );
        assert!(!fixture.journal_record_path().exists());
    }
}
