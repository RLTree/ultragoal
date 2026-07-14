use super::*;

#[test]
fn real_confined_effect_failure_reports_one_stable_causal_surface_and_remains_recoverable() {
    let fixture = Fixture::new("effect-failure");
    let package = fixture.bundle("0.0.12");
    let plan = fixture.install_plan(&package);
    let surfaces_parent = fixture.root.join("host-lifecycle/surfaces");
    let error = fixture
        .adapter
        .execute_with_test_hook(&plan, |point| {
            if point == DarwinTestPoint::BeforeCompareExchange(DarwinHostSurface::Installed) {
                std::fs::write(&surfaces_parent, b"parent-collision").unwrap();
            }
            DarwinTestControl::Continue
        })
        .unwrap_err();
    assert_eq!(error.id(), DarwinHostErrorId::EffectFailed);
    assert_eq!(error.surface(), Some(DarwinHostSurface::Installed));
    assert_eq!(error.to_string(), "confined host effect failed: Installed");

    std::fs::remove_file(surfaces_parent).unwrap();
    let recovered = fixture.adapter.recover(&plan).unwrap();
    assert_eq!(
        recovered.disposition(),
        DarwinHostTransactionDisposition::Recovered
    );
    assert_eq!(
        recovered.snapshot().diagnosis(),
        DarwinHostDiagnosis::Verified
    );
}

#[test]
fn an_inflight_journal_rejects_concurrent_execute_and_preserves_one_recovery_owner() {
    let fixture = Fixture::new("concurrent");
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

    let barrier = Arc::new(Barrier::new(9));
    let mut threads = Vec::new();
    for _ in 0..8 {
        let adapter = fixture.adapter.clone();
        let plan = plan.clone();
        let barrier = Arc::clone(&barrier);
        threads.push(std::thread::spawn(move || {
            barrier.wait();
            adapter.execute(&plan).unwrap_err().id()
        }));
    }
    barrier.wait();
    for thread in threads {
        assert_eq!(thread.join().unwrap(), DarwinHostErrorId::RecoveryRequired);
    }
    fixture.adapter.recover(&plan).unwrap();
}

#[test]
fn cancellation_claim_wins_before_recovery_and_leaves_no_unjournaled_surface() {
    let fixture = Fixture::new("cancellation-claim-wins");
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

    let (ready_tx, ready_rx) = mpsc::channel();
    let (resume_tx, resume_rx) = mpsc::channel();
    let recovery_adapter = fixture.adapter.clone();
    let recovery_plan = plan.clone();
    let recovery = std::thread::spawn(move || {
        recovery_adapter.recover_with_test_hook(&recovery_plan, |point| {
            if point == DarwinTestPoint::BeforeSurface(DarwinHostSurface::Installed) {
                ready_tx.send(()).unwrap();
                resume_rx.recv().unwrap();
            }
            DarwinTestControl::Continue
        })
    });

    ready_rx.recv().unwrap();
    let cancelled = fixture.adapter.cancel(&plan).unwrap();
    assert_eq!(
        cancelled.disposition(),
        DarwinHostTransactionDisposition::Cancelled
    );
    assert_eq!(
        cancelled.snapshot().diagnosis(),
        DarwinHostDiagnosis::Absent
    );
    assert!(cancelled.snapshot().recovery_plan_sha256().is_none());
    resume_tx.send(()).unwrap();
    assert_eq!(
        recovery.join().unwrap().unwrap_err().id(),
        DarwinHostErrorId::JournalConflict
    );
    let snapshot = fixture.adapter.query(&plan).unwrap();
    assert_eq!(snapshot.diagnosis(), DarwinHostDiagnosis::Absent);
    assert!(snapshot.recovery_plan_sha256().is_none());
    for surface in DarwinHostSurface::ALL {
        assert!(!fixture.record_path(surface).exists());
    }
}

#[test]
fn applying_claim_wins_before_cancellation_and_keeps_recovery_journaled() {
    let fixture = Fixture::new("applying-claim-wins");
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

    let (claimed_tx, claimed_rx) = mpsc::channel();
    let (resume_tx, resume_rx) = mpsc::channel();
    let recovery_adapter = fixture.adapter.clone();
    let recovery_plan = plan.clone();
    let recovery = std::thread::spawn(move || {
        recovery_adapter.recover_with_test_hook(&recovery_plan, |point| {
            if point == DarwinTestPoint::BeforeCompareExchange(DarwinHostSurface::Installed) {
                claimed_tx.send(()).unwrap();
                resume_rx.recv().unwrap();
            }
            DarwinTestControl::Continue
        })
    });

    claimed_rx.recv().unwrap();
    assert_eq!(
        fixture.adapter.cancel(&plan).unwrap_err().id(),
        DarwinHostErrorId::CancellationUnsafe
    );
    let pending = fixture.adapter.query(&plan).unwrap();
    assert_eq!(pending.diagnosis(), DarwinHostDiagnosis::RecoveryPending);
    assert_eq!(pending.recovery_plan_sha256(), Some(plan.plan_sha256()));
    resume_tx.send(()).unwrap();
    let recovered = recovery.join().unwrap().unwrap();
    assert_eq!(
        recovered.disposition(),
        DarwinHostTransactionDisposition::Recovered
    );
    assert_eq!(
        recovered.snapshot().diagnosis(),
        DarwinHostDiagnosis::Verified
    );
    assert!(recovered.snapshot().recovery_plan_sha256().is_none());
}
