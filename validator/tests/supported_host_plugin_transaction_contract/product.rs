use crate::host_lifecycle::{
    DarwinHostDiagnosis, DarwinHostErrorId, DarwinHostSurface, DarwinHostTransactionDisposition,
    DarwinSurfaceStatus,
};
use crate::transaction_fixture::{FOREIGN_CANDIDATE, Fixture, MARKETPLACE};
use std::process::Command;

#[test]
fn clean_install_materializes_seven_independently_observable_bound_surfaces() {
    let fixture = Fixture::new("clean-install");
    let package = fixture.bundle("0.0.12");
    let plan = fixture.install_plan(&package);
    let report = fixture.adapter.execute(&plan).unwrap();

    assert_eq!(
        report.disposition(),
        DarwinHostTransactionDisposition::Applied
    );
    assert_eq!(report.plan_sha256(), plan.plan_sha256());
    assert_eq!(report.snapshot().diagnosis(), DarwinHostDiagnosis::Verified);
    for surface in DarwinHostSurface::ALL {
        let row = report.snapshot().surface(surface);
        assert_eq!(row.surface(), surface);
        assert_eq!(row.status(), DarwinSurfaceStatus::Verified);
        assert!(row.tree_sha256().is_some());
        assert_eq!(row.observed_version(), Some("0.0.12"));
        assert!(fixture.record_path(surface).is_file());
    }
    let digests = DarwinHostSurface::ALL
        .map(|surface| report.snapshot().surface(surface).tree_sha256().unwrap())
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(digests.len(), DarwinHostSurface::ALL.len());
}

#[test]
fn versioned_update_requires_and_replaces_one_exact_prior_package_on_every_surface() {
    let fixture = Fixture::new("versioned-update");
    let previous = fixture.bundle_with_candidate("0.0.11", FOREIGN_CANDIDATE);
    let target = fixture.bundle("0.0.12");
    fixture
        .adapter
        .seed_candidate_for_test(&previous.snapshot)
        .unwrap();
    let plan = fixture
        .adapter
        .plan_update(&target.snapshot, &previous.snapshot, MARKETPLACE)
        .unwrap();
    let before = fixture.adapter.query(&plan).unwrap();
    for surface in DarwinHostSurface::ALL {
        assert_eq!(before.surface(surface).status(), DarwinSurfaceStatus::Stale);
        assert_eq!(before.surface(surface).observed_version(), Some("0.0.11"));
    }

    let report = fixture.adapter.execute(&plan).unwrap();
    assert_eq!(report.snapshot().diagnosis(), DarwinHostDiagnosis::Verified);
    for surface in DarwinHostSurface::ALL {
        assert_eq!(
            report.snapshot().surface(surface).observed_version(),
            Some("0.0.12")
        );
    }
}

#[test]
fn repeat_query_and_reinstall_are_byte_for_byte_zero_write_idempotent() {
    let fixture = Fixture::new("repeat-reinstall");
    let package = fixture.bundle("0.0.12");
    fixture
        .adapter
        .execute(&fixture.install_plan(&package))
        .unwrap();
    assert!(
        Command::new("git")
            .args(["init", "-q"])
            .current_dir(&fixture.root)
            .status()
            .unwrap()
            .success()
    );
    let reinstall = fixture.reinstall_plan(&package);
    let status_before = git_status(&fixture);
    let before = fixture.tree();
    let first = fixture.adapter.query(&reinstall).unwrap();
    let status_after_query = git_status(&fixture);
    let after_query = fixture.tree();
    let second = fixture.adapter.execute(&reinstall).unwrap();
    let status_after_reinstall = git_status(&fixture);
    let after_reinstall = fixture.tree();

    assert_eq!(first.diagnosis(), DarwinHostDiagnosis::Verified);
    assert_eq!(
        before, after_query,
        "query changed the recursive fixture tree"
    );
    assert_eq!(before, after_reinstall, "reinstall changed converged bytes");
    assert_eq!(
        status_before, status_after_query,
        "query changed same-scope Git status"
    );
    assert_eq!(
        status_before, status_after_reinstall,
        "reinstall changed same-scope Git status"
    );
    assert_eq!(
        second.disposition(),
        DarwinHostTransactionDisposition::AlreadyConverged
    );
}

fn git_status(fixture: &Fixture) -> Vec<u8> {
    let output = Command::new("git")
        .args(["status", "--porcelain=v2", "--untracked-files=all"])
        .current_dir(&fixture.root)
        .output()
        .unwrap();
    assert!(output.status.success());
    output.stdout
}

#[test]
fn uninstall_removes_each_surface_without_conflating_absence() {
    let fixture = Fixture::new("uninstall");
    let package = fixture.bundle("0.0.12");
    fixture
        .adapter
        .execute(&fixture.install_plan(&package))
        .unwrap();
    let uninstall = fixture
        .adapter
        .plan_uninstall(&package.snapshot, MARKETPLACE)
        .unwrap();
    let report = fixture.adapter.execute(&uninstall).unwrap();
    assert_eq!(report.snapshot().diagnosis(), DarwinHostDiagnosis::Absent);
    for surface in DarwinHostSurface::ALL {
        assert_eq!(
            report.snapshot().surface(surface).status(),
            DarwinSurfaceStatus::Absent
        );
        assert!(!fixture.record_path(surface).exists());
    }
}

#[test]
fn stale_cache_repair_changes_cache_only_and_preserves_six_verified_surfaces() {
    let fixture = Fixture::new("stale-cache");
    let stale = fixture.bundle("0.0.11");
    let target = fixture.bundle("0.0.12");
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
    let before = fixture.adapter.query(&plan).unwrap();
    assert_eq!(before.diagnosis(), DarwinHostDiagnosis::StaleCache);
    let preserved = DarwinHostSurface::ALL
        .into_iter()
        .filter(|surface| *surface != DarwinHostSurface::Cache)
        .map(|surface| {
            (
                surface,
                std::fs::read(fixture.record_path(surface)).unwrap(),
            )
        })
        .collect::<Vec<_>>();

    let report = fixture.adapter.execute(&plan).unwrap();
    assert_eq!(report.snapshot().diagnosis(), DarwinHostDiagnosis::Verified);
    for (surface, bytes) in preserved {
        assert_eq!(std::fs::read(fixture.record_path(surface)).unwrap(), bytes);
    }
}

#[test]
fn partial_and_dirty_states_are_diagnosed_and_refused_as_effect_preconditions() {
    let partial = Fixture::new("partial");
    let package = partial.bundle("0.0.12");
    partial
        .adapter
        .execute(&partial.install_plan(&package))
        .unwrap();
    partial.remove_surface(DarwinHostSurface::Runtime);
    let reinstall = partial.reinstall_plan(&package);
    assert_eq!(
        partial.adapter.query(&reinstall).unwrap().diagnosis(),
        DarwinHostDiagnosis::Partial
    );
    let error = partial.adapter.execute(&reinstall).unwrap_err();
    assert_eq!(error.id(), DarwinHostErrorId::SurfaceConflict);
    assert_eq!(error.surface(), Some(DarwinHostSurface::Runtime));

    let dirty = Fixture::new("dirty");
    let package = dirty.bundle("0.0.12");
    dirty
        .adapter
        .execute(&dirty.install_plan(&package))
        .unwrap();
    dirty.overwrite_record(DarwinHostSurface::Marketplace, b"not-json");
    let reinstall = dirty.reinstall_plan(&package);
    let snapshot = dirty.adapter.query(&reinstall).unwrap();
    assert_eq!(snapshot.diagnosis(), DarwinHostDiagnosis::Conflict);
    assert_eq!(snapshot.marketplace().status(), DarwinSurfaceStatus::Dirty);
    let error = dirty.adapter.execute(&reinstall).unwrap_err();
    assert_eq!(error.id(), DarwinHostErrorId::SurfaceConflict);
    assert_eq!(error.surface(), Some(DarwinHostSurface::Marketplace));
}

#[test]
fn unsupported_package_and_marketplace_are_rejected_before_any_host_write() {
    let fixture = Fixture::new("unsupported-bindings");
    let old = fixture.bundle("0.0.11");
    assert_eq!(
        fixture
            .adapter
            .plan_install(&old.snapshot, MARKETPLACE)
            .unwrap_err()
            .id(),
        DarwinHostErrorId::UnsupportedVersion
    );
    let supported = fixture.bundle("0.0.12");
    let before = fixture.tree();
    assert_eq!(
        fixture
            .adapter
            .plan_install(&supported.snapshot, "../rogue-marketplace")
            .unwrap_err()
            .id(),
        DarwinHostErrorId::InvalidMarketplace
    );
    assert_eq!(before, fixture.tree());
    for surface in DarwinHostSurface::ALL {
        assert!(!fixture.record_path(surface).exists());
    }
}
