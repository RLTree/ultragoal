use super::*;

#[test]
fn applying_claim_can_be_recovered_repeatedly_before_the_effect() {
    let fixture = Fixture::new("repeat-applying-recovery");
    let package = fixture.bundle("0.0.12");
    let plan = fixture.install_plan(&package);
    let interrupt_before_effect = |point| {
        if point == DarwinTestPoint::BeforeCompareExchange(DarwinHostSurface::Installed) {
            DarwinTestControl::Interrupt
        } else {
            DarwinTestControl::Continue
        }
    };

    assert_eq!(
        fixture
            .adapter
            .execute_with_test_hook(&plan, interrupt_before_effect)
            .unwrap_err()
            .id(),
        DarwinHostErrorId::Interrupted
    );
    assert_eq!(
        fixture
            .adapter
            .recover_with_test_hook(&plan, interrupt_before_effect)
            .unwrap_err()
            .id(),
        DarwinHostErrorId::Interrupted
    );
    assert!(!fixture.record_path(DarwinHostSurface::Installed).exists());
    assert_eq!(
        fixture.adapter.query(&plan).unwrap().diagnosis(),
        DarwinHostDiagnosis::RecoveryPending
    );

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
fn interrupted_cancellation_recovers_to_absent_without_any_surface_effect() {
    let fixture = Fixture::new("interrupted-cancellation-claim");
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
                if point == DarwinTestPoint::AfterCancellationClaim {
                    DarwinTestControl::Interrupt
                } else {
                    DarwinTestControl::Continue
                }
            })
            .unwrap_err()
            .id(),
        DarwinHostErrorId::Interrupted
    );
    assert_eq!(
        fixture.adapter.query(&plan).unwrap().diagnosis(),
        DarwinHostDiagnosis::RecoveryPending
    );
    let recovered = fixture.adapter.recover(&plan).unwrap();
    assert_eq!(
        recovered.disposition(),
        DarwinHostTransactionDisposition::Cancelled
    );
    assert_eq!(
        recovered.snapshot().diagnosis(),
        DarwinHostDiagnosis::Absent
    );
    assert!(recovered.snapshot().recovery_plan_sha256().is_none());
    for surface in DarwinHostSurface::ALL {
        assert!(!fixture.record_path(surface).exists());
    }
}
