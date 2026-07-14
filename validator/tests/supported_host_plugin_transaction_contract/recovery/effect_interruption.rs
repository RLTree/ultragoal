use super::*;

#[test]
fn interruption_before_each_individual_effect_recovers_deterministically() {
    for surface in DarwinHostSurface::ALL {
        let fixture = Fixture::new(&format!("crash-before-{surface:?}"));
        let package = fixture.bundle("0.0.12");
        let plan = fixture.install_plan(&package);
        let error = fixture
            .adapter
            .execute_with_test_hook(&plan, |point| {
                if point == DarwinTestPoint::BeforeSurface(surface) {
                    DarwinTestControl::Interrupt
                } else {
                    DarwinTestControl::Continue
                }
            })
            .unwrap_err();
        assert_eq!(error.id(), DarwinHostErrorId::Interrupted);
        assert_eq!(
            fixture.adapter.query(&plan).unwrap().diagnosis(),
            DarwinHostDiagnosis::RecoveryPending
        );
        assert_eq!(
            fixture.adapter.execute(&plan).unwrap_err().id(),
            DarwinHostErrorId::RecoveryRequired
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
}

#[test]
fn interruption_after_each_individual_effect_recovers_without_reapplying_prior_surfaces() {
    for surface in DarwinHostSurface::ALL {
        let fixture = Fixture::new(&format!("crash-after-{surface:?}"));
        let package = fixture.bundle("0.0.12");
        let plan = fixture.install_plan(&package);
        let error = fixture
            .adapter
            .execute_with_test_hook(&plan, |point| {
                if point == DarwinTestPoint::AfterSurface(surface) {
                    DarwinTestControl::Interrupt
                } else {
                    DarwinTestControl::Continue
                }
            })
            .unwrap_err();
        assert_eq!(error.id(), DarwinHostErrorId::Interrupted);
        let bytes_after_crash = std::fs::read(fixture.record_path(surface)).unwrap();
        let recovered = fixture.adapter.recover(&plan).unwrap();
        assert_eq!(
            recovered.disposition(),
            DarwinHostTransactionDisposition::Recovered
        );
        assert_eq!(
            std::fs::read(fixture.record_path(surface)).unwrap(),
            bytes_after_crash
        );
        assert_eq!(
            recovered.snapshot().diagnosis(),
            DarwinHostDiagnosis::Verified
        );
    }
}

#[test]
fn cancellation_is_safe_before_effects_and_rejected_after_any_effect() {
    let safe = Fixture::new("cancel-before");
    let package = safe.bundle("0.0.12");
    let plan = safe.install_plan(&package);
    safe.adapter
        .execute_with_test_hook(&plan, |point| {
            if point == DarwinTestPoint::BeforeSurface(DarwinHostSurface::Installed) {
                DarwinTestControl::Interrupt
            } else {
                DarwinTestControl::Continue
            }
        })
        .unwrap_err();
    let cancelled = safe.adapter.cancel(&plan).unwrap();
    assert_eq!(
        cancelled.disposition(),
        DarwinHostTransactionDisposition::Cancelled
    );
    assert_eq!(
        cancelled.snapshot().diagnosis(),
        DarwinHostDiagnosis::Absent
    );

    let unsafe_cancel = Fixture::new("cancel-after");
    let package = unsafe_cancel.bundle("0.0.12");
    let plan = unsafe_cancel.install_plan(&package);
    unsafe_cancel
        .adapter
        .execute_with_test_hook(&plan, |point| {
            if point == DarwinTestPoint::AfterSurface(DarwinHostSurface::Installed) {
                DarwinTestControl::Interrupt
            } else {
                DarwinTestControl::Continue
            }
        })
        .unwrap_err();
    let error = unsafe_cancel.adapter.cancel(&plan).unwrap_err();
    assert_eq!(error.id(), DarwinHostErrorId::CancellationUnsafe);
    assert_eq!(error.surface(), Some(DarwinHostSurface::Installed));
    unsafe_cancel.adapter.recover(&plan).unwrap();
}
