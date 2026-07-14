use super::*;

#[test]
fn toctou_mutation_between_journal_and_effect_fails_closed_with_stable_surface_diagnostic() {
    let fixture = Fixture::new("toctou");
    let previous = fixture.bundle("0.0.11");
    let target = fixture.bundle("0.0.12");
    fixture
        .adapter
        .seed_candidate_for_test(&previous.snapshot)
        .unwrap();
    let plan = fixture
        .adapter
        .plan_update(&target.snapshot, &previous.snapshot, MARKETPLACE)
        .unwrap();
    let installed = fixture.record_path(DarwinHostSurface::Installed);
    let error = fixture
        .adapter
        .execute_with_test_hook(&plan, |point| {
            if point == DarwinTestPoint::BeforeSurface(DarwinHostSurface::Installed) {
                std::fs::write(&installed, b"attacker-changed-the-observation").unwrap();
            }
            DarwinTestControl::Continue
        })
        .unwrap_err();
    assert_eq!(error.id(), DarwinHostErrorId::SurfaceConflict);
    assert_eq!(error.surface(), Some(DarwinHostSurface::Installed));
    assert_eq!(
        fixture.adapter.query(&plan).unwrap().diagnosis(),
        DarwinHostDiagnosis::RecoveryPending
    );
    assert_eq!(
        fixture.adapter.recover(&plan).unwrap_err().id(),
        DarwinHostErrorId::SurfaceConflict
    );
}

#[test]
fn mutation_between_idle_admission_and_reservation_refuses_without_a_journal_then_allows_use() {
    let fixture = Fixture::new("idle-admission-reservation-mutation");
    let terminal =
        fixture.bundle_with_candidate("0.0.12", crate::transaction_fixture::FOREIGN_CANDIDATE);
    let substituted = fixture.bundle("0.0.12");
    fixture
        .adapter
        .execute(&fixture.install_plan(&terminal))
        .unwrap();
    let plan = fixture
        .adapter
        .plan_uninstall(&terminal.snapshot, MARKETPLACE)
        .unwrap();
    let mut attacked = None;
    let attacker = fixture.adapter.clone();
    let error = fixture
        .adapter
        .execute_with_test_hook(&plan, |point| {
            if point == DarwinTestPoint::BeforeJournalReservation {
                for surface in DarwinHostSurface::ALL {
                    attacker
                        .replace_surface_for_test(surface, &substituted.snapshot)
                        .unwrap();
                }
                attacked = Some(fixture.tree());
            }
            DarwinTestControl::Continue
        })
        .unwrap_err();
    assert_eq!(error.id(), DarwinHostErrorId::SurfaceConflict);
    assert_eq!(fixture.tree(), attacked.unwrap());
    assert!(!fixture.journal_record_path().exists());

    for surface in DarwinHostSurface::ALL {
        fixture
            .adapter
            .replace_surface_for_test(surface, &terminal.snapshot)
            .unwrap();
    }
    assert_eq!(
        fixture
            .adapter
            .execute(&plan)
            .unwrap()
            .snapshot()
            .diagnosis(),
        DarwinHostDiagnosis::Absent
    );
}

#[test]
fn concurrent_mutation_after_reservation_is_removed_without_an_effect_then_allows_use() {
    let fixture = Fixture::new("post-reservation-concurrent-mutation");
    let target = fixture.bundle("0.0.12");
    let stale = fixture.bundle("0.0.11");
    let substituted =
        fixture.bundle_with_candidate("0.0.12", crate::transaction_fixture::FOREIGN_CANDIDATE);
    fixture
        .adapter
        .execute(&fixture.install_plan(&target))
        .unwrap();
    fixture
        .adapter
        .replace_surface_for_test(DarwinHostSurface::Cache, &stale.snapshot)
        .unwrap();
    let plan = fixture
        .adapter
        .plan_repair_cache(&target.snapshot, &stale.snapshot, MARKETPLACE)
        .unwrap();
    let before = fixture.tree();
    let cache_before = fs::read(fixture.record_path(DarwinHostSurface::Cache)).unwrap();
    let lineage_before = fs::read(fixture.lineage_record_path()).unwrap();
    let anchor_before = fs::read(fixture.lineage_anchor_record_path()).unwrap();
    let (mutate_tx, mutate_rx) = mpsc::channel();
    let (done_tx, done_rx) = mpsc::channel();
    let mutator = fixture.adapter.clone();
    let substituted_snapshot = substituted.snapshot.clone();
    let mutation = std::thread::spawn(move || {
        mutate_rx.recv().unwrap();
        mutator
            .replace_surface_for_test(DarwinHostSurface::Runtime, &substituted_snapshot)
            .unwrap();
        done_tx.send(()).unwrap();
    });

    let error = fixture
        .adapter
        .execute_with_test_hook(&plan, |point| {
            if point == DarwinTestPoint::AfterJournalReservation {
                mutate_tx.send(()).unwrap();
                done_rx.recv().unwrap();
            }
            DarwinTestControl::Continue
        })
        .unwrap_err();
    mutation.join().unwrap();
    assert_eq!(error.id(), DarwinHostErrorId::LineageConflict);
    assert!(!fixture.journal_record_path().exists());
    assert_eq!(
        fs::read(fixture.record_path(DarwinHostSurface::Cache)).unwrap(),
        cache_before
    );
    assert_eq!(
        fs::read(fixture.lineage_record_path()).unwrap(),
        lineage_before
    );
    assert_eq!(
        fs::read(fixture.lineage_anchor_record_path()).unwrap(),
        anchor_before
    );

    fixture
        .adapter
        .replace_surface_for_test(DarwinHostSurface::Runtime, &target.snapshot)
        .unwrap();
    assert_eq!(fixture.tree(), before);
    assert_eq!(
        fixture
            .adapter
            .execute(&plan)
            .unwrap()
            .snapshot()
            .diagnosis(),
        DarwinHostDiagnosis::Verified
    );
}

#[test]
fn repair_non_step_mutation_at_effect_boundary_refuses_then_recovers() {
    let fixture = Fixture::new("repair-effect-boundary-mutation");
    let target = fixture.bundle("0.0.12");
    let stale = fixture.bundle("0.0.11");
    let substituted =
        fixture.bundle_with_candidate("0.0.12", crate::transaction_fixture::FOREIGN_CANDIDATE);
    fixture
        .adapter
        .execute(&fixture.install_plan(&target))
        .unwrap();
    fixture
        .adapter
        .replace_surface_for_test(DarwinHostSurface::Cache, &stale.snapshot)
        .unwrap();
    let plan = fixture
        .adapter
        .plan_repair_cache(&target.snapshot, &stale.snapshot, MARKETPLACE)
        .unwrap();
    let cache_before = fs::read(fixture.record_path(DarwinHostSurface::Cache)).unwrap();
    let attacker = fixture.adapter.clone();

    let error = fixture
        .adapter
        .execute_with_test_hook(&plan, |point| {
            if point == DarwinTestPoint::BeforeSurface(DarwinHostSurface::Cache) {
                attacker
                    .replace_surface_for_test(DarwinHostSurface::Runtime, &substituted.snapshot)
                    .unwrap();
            }
            DarwinTestControl::Continue
        })
        .unwrap_err();
    assert_eq!(error.id(), DarwinHostErrorId::SurfaceConflict);
    assert_eq!(error.surface(), Some(DarwinHostSurface::Runtime));
    assert_eq!(
        fs::read(fixture.record_path(DarwinHostSurface::Cache)).unwrap(),
        cache_before
    );
    assert!(fixture.journal_record_path().is_file());

    fixture
        .adapter
        .replace_surface_for_test(DarwinHostSurface::Runtime, &target.snapshot)
        .unwrap();
    assert_eq!(
        fixture
            .adapter
            .recover(&plan)
            .unwrap()
            .snapshot()
            .diagnosis(),
        DarwinHostDiagnosis::Verified
    );
}
