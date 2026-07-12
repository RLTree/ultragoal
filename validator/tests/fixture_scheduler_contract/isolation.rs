use crate::fixture_scheduler::*;
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
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

#[test]
fn parallel_leases_have_disjoint_roots_and_all_resource_namespaces() {
    let root = root();
    let mut scheduler = FixtureScheduler::new(&root);
    let ids = scheduler.schedule([spec("alpha"), spec("beta")]).unwrap();
    let alpha = scheduler.run(&ids[0]).unwrap();
    let beta = scheduler.run(&ids[1]).unwrap();
    assert_ne!(alpha.lease.root(), beta.lease.root());
    assert_eq!(alpha.environment.len(), 7);
    for (key, value) in &alpha.environment {
        assert_ne!(value, beta.environment.get(key).unwrap());
    }
    assert!(alpha.lease.root().join(".fixture-lease").is_file());
    for id in ids {
        assert!(matches!(
            scheduler.finish(&id, ObservedOutcome::pass(2)),
            Err(FixtureScheduleError::Integrity(_))
        ));
    }
    assert!(!root.join("alpha").exists());
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
    assert_eq!(
        scheduler.recover(&id).unwrap(),
        RunDisposition::CausalFailure
    );
    assert_eq!(scheduler.active_count(), 0);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn newly_scheduled_lease_is_returned_even_when_another_lease_sorts_later() {
    let root = root();
    let mut scheduler = FixtureScheduler::new(&root);
    let later = scheduler.schedule([spec("zeta")]).unwrap().pop().unwrap();
    let earlier = scheduler.schedule([spec("alpha")]).unwrap().pop().unwrap();
    assert!(earlier.starts_with("alpha-"));
    assert!(
        scheduler
            .finish(&earlier, ObservedOutcome::pass(2))
            .is_err()
    );
    assert!(scheduler.finish(&later, ObservedOutcome::pass(2)).is_err());
    fs::remove_dir_all(root).unwrap();
}
