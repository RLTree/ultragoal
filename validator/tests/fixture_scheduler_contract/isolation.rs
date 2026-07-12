use crate::fixture_scheduler::*;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn root() -> PathBuf {
    std::env::temp_dir().join(format!(
        "hul-fixture-isolation-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}
fn spec(id: &str) -> FixtureSpec {
    FixtureSpec::new(
        id,
        FixtureKind::Positive,
        "isolation",
        BTreeSet::from([
            ResourceKind::File,
            ResourceKind::Env,
            ResourceKind::Process,
            ResourceKind::Port,
            ResourceKind::Cache,
            ResourceKind::Telemetry,
            ResourceKind::Install,
        ]),
        ExpectedOutcome::pass(2),
        false,
    )
    .unwrap()
}

fn assert_exact_namespaces(
    run: &FixtureRun,
    repository_root: &Path,
    lease_id: &str,
    fixture_id: &str,
) {
    let lease_root = repository_root.join(lease_id);
    assert_eq!(run.fixture.id, fixture_id);
    assert_eq!(run.lease.id(), lease_id);
    assert_eq!(run.lease.fixture_id(), fixture_id);
    assert_eq!(run.lease.root(), lease_root);
    assert_eq!(run.lease.bindings().len(), 7);
    assert_eq!(run.environment.len(), 7);
    assert_eq!(
        fs::read_to_string(lease_root.join(".fixture-lease")).unwrap(),
        format!("fixture={fixture_id}\n")
    );

    for kind in [
        ResourceKind::File,
        ResourceKind::Env,
        ResourceKind::Process,
        ResourceKind::Port,
        ResourceKind::Cache,
        ResourceKind::Telemetry,
        ResourceKind::Install,
    ] {
        let binding = run
            .lease
            .bindings()
            .iter()
            .find(|binding| binding.kind == kind)
            .unwrap();
        let environment_key = format!("HUL_FIXTURE_{}", kind.label().to_ascii_uppercase());
        assert_eq!(run.environment.get(&environment_key), Some(&binding.key));

        let namespace = lease_root.join(kind.label());
        assert!(namespace.is_dir(), "missing namespace {namespace:?}");
        if kind == ResourceKind::Port {
            assert_eq!(
                fs::read_to_string(namespace.join("reservation")).unwrap(),
                binding.key
            );
        } else {
            assert_eq!(binding.key, namespace.to_str().unwrap());
        }
    }
}

fn assert_recovery_required(
    scheduler: &FixtureScheduler,
    repository_root: &Path,
    lease_id: &str,
    fixture_id: &str,
) {
    let retained = scheduler.run(lease_id).unwrap();
    assert_eq!(
        retained.lease.disposition(),
        &LeaseDisposition::RecoveryRequired
    );
    assert!(!retained.is_active());
    assert_exact_namespaces(retained, repository_root, lease_id, fixture_id);
}

#[cfg(not(target_os = "freebsd"))]
fn finish_and_recover_on_platform(
    scheduler: &mut FixtureScheduler,
    repository_root: &Path,
    lease_id: &str,
    fixture_id: &str,
    expected_active_count: usize,
) {
    assert_eq!(
        scheduler
            .finish(lease_id, ObservedOutcome::pass(2))
            .unwrap(),
        RunDisposition::CleanupFailure
    );
    assert_eq!(scheduler.active_count(), expected_active_count);
    assert_recovery_required(scheduler, repository_root, lease_id, fixture_id);

    assert_eq!(
        scheduler.recover(lease_id).unwrap(),
        RunDisposition::CleanupFailure
    );
    assert_eq!(scheduler.active_count(), expected_active_count);
    assert_recovery_required(scheduler, repository_root, lease_id, fixture_id);
}

#[cfg(target_os = "freebsd")]
fn finish_and_recover_on_platform(
    scheduler: &mut FixtureScheduler,
    repository_root: &Path,
    lease_id: &str,
    _fixture_id: &str,
    expected_active_count: usize,
) {
    assert!(matches!(
        scheduler.finish(lease_id, ObservedOutcome::pass(2)),
        Err(FixtureScheduleError::Integrity(_))
    ));
    assert_eq!(scheduler.active_count(), expected_active_count);
    assert!(!repository_root.join(lease_id).exists());
    assert!(matches!(
        scheduler.recover(lease_id),
        Err(FixtureScheduleError::UnknownLease(observed)) if observed == lease_id
    ));
}

#[test]
fn parallel_leases_have_disjoint_roots_and_all_resource_namespaces() {
    let root = root();
    let mut scheduler = FixtureScheduler::new(&root);
    let ids = scheduler.schedule([spec("alpha"), spec("beta")]).unwrap();
    let alpha = scheduler.run(&ids[0]).unwrap();
    let beta = scheduler.run(&ids[1]).unwrap();
    assert_ne!(alpha.lease.root(), beta.lease.root());
    assert_exact_namespaces(alpha, &root, &ids[0], "alpha");
    assert_exact_namespaces(beta, &root, &ids[1], "beta");
    for (key, value) in &alpha.environment {
        assert_ne!(value, beta.environment.get(key).unwrap());
    }
    #[cfg(target_os = "freebsd")]
    let expected_after_alpha = 1;
    #[cfg(not(target_os = "freebsd"))]
    let expected_after_alpha = 2;
    finish_and_recover_on_platform(
        &mut scheduler,
        &root,
        &ids[0],
        "alpha",
        expected_after_alpha,
    );
    #[cfg(target_os = "freebsd")]
    let expected_after_beta = 0;
    #[cfg(not(target_os = "freebsd"))]
    let expected_after_beta = 2;
    finish_and_recover_on_platform(&mut scheduler, &root, &ids[1], "beta", expected_after_beta);
    drop(scheduler);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn concurrent_duplicate_fixture_lease_is_rejected_and_recovery_releases_it() {
    let root = root();
    let mut scheduler = FixtureScheduler::new(&root);
    let id = scheduler.schedule([spec("same")]).unwrap().pop().unwrap();
    assert!(matches!(
        scheduler.schedule([spec("same")]),
        Err(FixtureScheduleError::Collision(_))
    ));
    assert_exact_namespaces(scheduler.run(&id).unwrap(), &root, &id, "same");

    #[cfg(target_os = "freebsd")]
    {
        assert_eq!(
            scheduler.recover(&id).unwrap(),
            RunDisposition::CausalFailure
        );
        assert_eq!(scheduler.active_count(), 0);
        assert!(!root.join(&id).exists());
        assert!(matches!(
            scheduler.recover(&id),
            Err(FixtureScheduleError::UnknownLease(observed)) if observed == id
        ));
    }
    #[cfg(not(target_os = "freebsd"))]
    {
        assert_eq!(
            scheduler.recover(&id).unwrap(),
            RunDisposition::CleanupFailure
        );
        assert_eq!(scheduler.active_count(), 1);
        assert_recovery_required(&scheduler, &root, &id, "same");
        assert_eq!(
            scheduler.recover(&id).unwrap(),
            RunDisposition::CleanupFailure
        );
        assert_eq!(scheduler.active_count(), 1);
        assert_recovery_required(&scheduler, &root, &id, "same");
        assert!(matches!(
            scheduler.schedule([spec("same")]),
            Err(FixtureScheduleError::Collision(_))
        ));
    }
    drop(scheduler);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn newly_scheduled_lease_is_returned_even_when_another_lease_sorts_later() {
    let root = root();
    let mut scheduler = FixtureScheduler::new(&root);
    let later = scheduler.schedule([spec("zeta")]).unwrap().pop().unwrap();
    let earlier = scheduler.schedule([spec("alpha")]).unwrap().pop().unwrap();
    assert!(earlier.starts_with("alpha-"));
    assert_eq!(
        scheduler.run(&later).unwrap().lease.root(),
        root.join(&later)
    );
    assert_eq!(
        scheduler.run(&earlier).unwrap().lease.root(),
        root.join(&earlier)
    );
    assert_exact_namespaces(scheduler.run(&later).unwrap(), &root, &later, "zeta");
    assert_exact_namespaces(scheduler.run(&earlier).unwrap(), &root, &earlier, "alpha");

    #[cfg(target_os = "freebsd")]
    let expected_after_earlier = 1;
    #[cfg(not(target_os = "freebsd"))]
    let expected_after_earlier = 2;
    finish_and_recover_on_platform(
        &mut scheduler,
        &root,
        &earlier,
        "alpha",
        expected_after_earlier,
    );
    #[cfg(target_os = "freebsd")]
    let expected_after_later = 0;
    #[cfg(not(target_os = "freebsd"))]
    let expected_after_later = 2;
    finish_and_recover_on_platform(&mut scheduler, &root, &later, "zeta", expected_after_later);
    drop(scheduler);
    fs::remove_dir_all(root).unwrap();
}
