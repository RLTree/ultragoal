use super::*;

#[test]
fn terminal_lineage_rollback_is_rejected_without_surface_mutation() {
    let fixture = Fixture::new("terminal-lineage-rollback");
    let package = fixture.bundle("0.0.12");
    let install = fixture.install_plan(&package);
    fixture.adapter.execute(&install).unwrap();
    let old_lineage = fs::read(fixture.lineage_record_path()).unwrap();
    let uninstall = fixture
        .adapter
        .plan_uninstall(&package.snapshot, MARKETPLACE)
        .unwrap();
    fixture.adapter.execute(&uninstall).unwrap();
    let current_lineage = fs::read(fixture.lineage_record_path()).unwrap();
    assert_ne!(old_lineage, current_lineage);

    fs::write(fixture.lineage_record_path(), old_lineage).unwrap();
    let before = fixture.tree();
    assert_eq!(
        fixture.adapter.query(&uninstall).unwrap_err().id(),
        DarwinHostErrorId::LineageConflict
    );
    assert_eq!(fixture.tree(), before);
    for surface in DarwinHostSurface::ALL {
        assert!(!fixture.record_path(surface).exists());
    }
}

#[test]
fn coherent_terminal_predecessor_substitution_cannot_be_laundered_by_update() {
    let fixture = Fixture::new("terminal-predecessor-update-substitution");
    let terminal = fixture.bundle_with_candidate("0.0.12", FOREIGN_CANDIDATE);
    fixture
        .adapter
        .execute(&fixture.install_plan(&terminal))
        .unwrap();

    let substituted_prior = fixture.bundle("0.0.11");
    let successor = fixture.bundle("0.0.12");
    replace_all_surfaces(&fixture, &substituted_prior);
    let plan = fixture
        .adapter
        .plan_update(
            &successor.snapshot,
            &substituted_prior.snapshot,
            MARKETPLACE,
        )
        .unwrap();
    assert_eq!(
        fixture.adapter.query(&plan).unwrap().diagnosis(),
        DarwinHostDiagnosis::Conflict
    );
    let before = fixture.tree();

    let error = fixture.adapter.execute(&plan).unwrap_err();
    assert_eq!(error.id(), DarwinHostErrorId::LineageConflict);
    assert_eq!(fixture.tree(), before);
    assert!(!fixture.journal_record_path().exists());

    replace_all_surfaces(&fixture, &terminal);
    assert_eq!(
        fixture
            .adapter
            .query(&fixture.reinstall_plan(&terminal))
            .unwrap()
            .diagnosis(),
        DarwinHostDiagnosis::Verified
    );
}

#[test]
fn coherent_terminal_predecessor_substitution_cannot_be_laundered_by_repair() {
    let fixture = Fixture::new("terminal-predecessor-repair-substitution");
    let terminal = fixture.bundle_with_candidate("0.0.12", FOREIGN_CANDIDATE);
    let terminal_stale = fixture.bundle_with_candidate("0.0.11", FOREIGN_CANDIDATE);
    fixture
        .adapter
        .execute(&fixture.install_plan(&terminal))
        .unwrap();

    let substituted_target = fixture.bundle("0.0.12");
    let substituted_stale = fixture.bundle("0.0.11");
    replace_all_surfaces(&fixture, &substituted_target);
    fixture
        .adapter
        .replace_surface_for_test(DarwinHostSurface::Cache, &substituted_stale.snapshot)
        .unwrap();
    let substituted_plan = fixture
        .adapter
        .plan_repair_cache(
            &substituted_target.snapshot,
            &substituted_stale.snapshot,
            MARKETPLACE,
        )
        .unwrap();
    assert_eq!(
        fixture
            .adapter
            .query(&substituted_plan)
            .unwrap()
            .diagnosis(),
        DarwinHostDiagnosis::StaleCache
    );
    let before = fixture.tree();

    let error = fixture.adapter.execute(&substituted_plan).unwrap_err();
    assert_eq!(error.id(), DarwinHostErrorId::LineageConflict);
    assert_eq!(fixture.tree(), before);
    assert!(!fixture.journal_record_path().exists());

    replace_all_surfaces(&fixture, &terminal);
    fixture
        .adapter
        .replace_surface_for_test(DarwinHostSurface::Cache, &terminal_stale.snapshot)
        .unwrap();
    let valid = fixture
        .adapter
        .plan_repair_cache(&terminal.snapshot, &terminal_stale.snapshot, MARKETPLACE)
        .unwrap();
    assert_eq!(
        fixture
            .adapter
            .execute(&valid)
            .unwrap()
            .snapshot()
            .diagnosis(),
        DarwinHostDiagnosis::Verified
    );
}

#[test]
fn coherent_terminal_predecessor_substitution_cannot_be_laundered_by_uninstall() {
    let fixture = Fixture::new("terminal-predecessor-uninstall-substitution");
    let terminal = fixture.bundle_with_candidate("0.0.12", FOREIGN_CANDIDATE);
    fixture
        .adapter
        .execute(&fixture.install_plan(&terminal))
        .unwrap();

    let substituted = fixture.bundle("0.0.12");
    replace_all_surfaces(&fixture, &substituted);
    let substituted_plan = fixture
        .adapter
        .plan_uninstall(&substituted.snapshot, MARKETPLACE)
        .unwrap();
    let before = fixture.tree();

    let error = fixture.adapter.execute(&substituted_plan).unwrap_err();
    assert_eq!(error.id(), DarwinHostErrorId::LineageConflict);
    assert_eq!(fixture.tree(), before);
    assert!(!fixture.journal_record_path().exists());

    replace_all_surfaces(&fixture, &terminal);
    let valid = fixture
        .adapter
        .plan_uninstall(&terminal.snapshot, MARKETPLACE)
        .unwrap();
    assert_eq!(
        fixture
            .adapter
            .execute(&valid)
            .unwrap()
            .snapshot()
            .diagnosis(),
        DarwinHostDiagnosis::Absent
    );
}

#[test]
fn cancelled_terminal_lineage_binds_observed_predecessor_without_asserting_target() {
    let fixture = Fixture::new("cancelled-terminal-predecessor-binding");
    let prior = fixture.bundle("0.0.11");
    let target = fixture.bundle("0.0.12");
    fixture
        .adapter
        .seed_candidate_for_test(&prior.snapshot)
        .unwrap();
    let plan = fixture
        .adapter
        .plan_update(&target.snapshot, &prior.snapshot, MARKETPLACE)
        .unwrap();
    fixture
        .adapter
        .execute_with_test_hook(&plan, |point| {
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
        fixture
            .adapter
            .cancel(&plan)
            .unwrap()
            .snapshot()
            .diagnosis(),
        DarwinHostDiagnosis::Conflict
    );

    let substituted_prior = fixture.bundle_with_candidate("0.0.11", FOREIGN_CANDIDATE);
    let substituted_target = fixture.bundle_with_candidate("0.0.12", FOREIGN_CANDIDATE);
    replace_all_surfaces(&fixture, &substituted_prior);
    let substituted_plan = fixture
        .adapter
        .plan_update(
            &substituted_target.snapshot,
            &substituted_prior.snapshot,
            MARKETPLACE,
        )
        .unwrap();
    let before = fixture.tree();
    assert_eq!(
        fixture.adapter.execute(&substituted_plan).unwrap_err().id(),
        DarwinHostErrorId::LineageConflict
    );
    assert_eq!(fixture.tree(), before);
    assert!(!fixture.journal_record_path().exists());

    replace_all_surfaces(&fixture, &prior);
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
