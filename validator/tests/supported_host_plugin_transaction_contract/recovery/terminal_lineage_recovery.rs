use super::*;

#[test]
fn stale_pending_applying_and_cancelling_journals_never_supersede_terminal_lineage() {
    for (label, point, cancelling) in [
        (
            "pending",
            DarwinTestPoint::BeforeSurface(DarwinHostSurface::Installed),
            false,
        ),
        (
            "applying",
            DarwinTestPoint::BeforeCompareExchange(DarwinHostSurface::Installed),
            false,
        ),
        (
            "cancelling",
            DarwinTestPoint::BeforeSurface(DarwinHostSurface::Installed),
            true,
        ),
    ] {
        let fixture = Fixture::new(&format!("stale-{label}-journal"));
        let package = fixture.bundle("0.0.12");
        let install = fixture.install_plan(&package);
        fixture
            .adapter
            .execute_with_test_hook(&install, |current| {
                if current == point {
                    DarwinTestControl::Interrupt
                } else {
                    DarwinTestControl::Continue
                }
            })
            .unwrap_err();
        if cancelling {
            fixture
                .adapter
                .cancel_with_test_hook(&install, |current| {
                    if current == DarwinTestPoint::AfterCancellationClaim {
                        DarwinTestControl::Interrupt
                    } else {
                        DarwinTestControl::Continue
                    }
                })
                .unwrap_err();
        }
        let stale = fs::read(fixture.journal_record_path()).unwrap();

        let first = fixture.adapter.recover(&install).unwrap();
        if cancelling {
            assert_eq!(
                first.disposition(),
                DarwinHostTransactionDisposition::Cancelled
            );
            fixture.adapter.execute(&install).unwrap();
        } else {
            assert_eq!(
                first.disposition(),
                DarwinHostTransactionDisposition::Recovered
            );
        }
        let uninstall = fixture
            .adapter
            .plan_uninstall(&package.snapshot, MARKETPLACE)
            .unwrap();
        fixture.adapter.execute(&uninstall).unwrap();
        assert_eq!(
            fixture.adapter.query(&uninstall).unwrap().diagnosis(),
            DarwinHostDiagnosis::Absent
        );

        let journal = fixture.journal_record_path();
        fs::create_dir_all(journal.parent().unwrap()).unwrap();
        fs::write(&journal, stale).unwrap();
        fs::set_permissions(&journal, fs::Permissions::from_mode(0o644)).unwrap();
        let before = fixture.tree();
        let error = match reopen(&fixture) {
            Ok(_) => panic!("stale journal reopened a terminal transaction"),
            Err(error) => error,
        };
        assert_eq!(error.id(), DarwinHostErrorId::LineageConflict);
        assert_eq!(fixture.tree(), before);
        for surface in DarwinHostSurface::ALL {
            assert!(!fixture.record_path(surface).exists());
        }
    }
}

#[test]
fn concurrent_terminal_lineage_cas_has_one_winner_and_one_verified_result() {
    let fixture = Fixture::new("lineage-concurrent-cas");
    let package = fixture.bundle("0.0.12");
    let plan = fixture.install_plan(&package);
    fixture
        .adapter
        .execute_with_test_hook(&plan, |point| {
            if point == DarwinTestPoint::AfterProgress(DarwinHostSurface::Runtime) {
                DarwinTestControl::Interrupt
            } else {
                DarwinTestControl::Continue
            }
        })
        .unwrap_err();

    let barrier = Arc::new(Barrier::new(9));
    let mut threads = Vec::new();
    for _ in 0..8 {
        let adapter = fixture.adapter.clone();
        let plan = plan.clone();
        let barrier = Arc::clone(&barrier);
        threads.push(std::thread::spawn(move || {
            barrier.wait();
            adapter.recover(&plan)
        }));
    }
    barrier.wait();
    let mut successes = 0;
    for thread in threads {
        match thread.join().unwrap() {
            Ok(report) => {
                successes += 1;
                assert_eq!(
                    report.disposition(),
                    DarwinHostTransactionDisposition::Recovered
                );
            }
            Err(error) => assert!(
                matches!(
                    error.id(),
                    DarwinHostErrorId::JournalConflict | DarwinHostErrorId::LineageConflict
                ),
                "unexpected concurrent terminalization error: {error:?}"
            ),
        }
    }
    assert_eq!(successes, 1);
    assert_eq!(
        fixture.adapter.query(&plan).unwrap().diagnosis(),
        DarwinHostDiagnosis::Verified
    );
    assert!(!fixture.journal_record_path().exists());
    assert!(fixture.lineage_record_path().is_file());
}
