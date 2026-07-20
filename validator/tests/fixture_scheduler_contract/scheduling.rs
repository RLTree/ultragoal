use crate::fixture_scheduler::*;
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn root(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("hul-fixture-{name}-{nonce}"))
}

fn spec(id: &str, kind: FixtureKind, expected: ExpectedOutcome, flaky: bool) -> FixtureSpec {
    FixtureSpec::new(
        id,
        kind,
        "proof-target",
        BTreeSet::from([ResourceKind::File, ResourceKind::Cache]),
        expected,
        flaky,
    )
    .unwrap()
}
#[test]
fn scheduling_is_deterministic_and_negative_controls_are_causal() {
    let root = root("ordering");
    let mut scheduler = FixtureScheduler::new(&root);
    let ids = scheduler
        .schedule(vec![
            spec(
                "z-negative",
                FixtureKind::Negative,
                ExpectedOutcome::causal_failure("missing-proof", 1),
                false,
            ),
            spec(
                "a-positive",
                FixtureKind::Positive,
                ExpectedOutcome::pass(3),
                false,
            ),
        ])
        .unwrap();
    assert!(ids[0].starts_with("a-positive-"));
    assert!(ids[1].starts_with("z-negative-"));
    for id in &ids {
        let result = scheduler.finish(id, ObservedOutcome::pass(3));
        #[cfg(target_os = "freebsd")]
        assert!(matches!(result, Err(FixtureScheduleError::Integrity(_))));
        #[cfg(not(target_os = "freebsd"))]
        {
            assert_eq!(result.unwrap(), RunDisposition::CleanupFailure);
            let retained = scheduler.run(id).unwrap();
            assert_eq!(
                retained.lease.disposition(),
                &LeaseDisposition::RecoveryRequired
            );
            assert!(!retained.is_active());
        }
    }
    #[cfg(target_os = "freebsd")]
    assert_eq!(scheduler.active_count(), 0);
    #[cfg(not(target_os = "freebsd"))]
    assert_eq!(scheduler.active_count(), 2);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn stale_tamper_recovery_and_reward_hacking_are_first_class_controls() {
    let root = root("controls");
    let mut scheduler = FixtureScheduler::new(&root);
    let cases = [
        ("tamper", FixtureKind::Tamper, "tampered-metadata"),
        ("stale", FixtureKind::Stale, "stale-capture"),
        ("recovery", FixtureKind::Recovery, "recovered-cleanup"),
        ("reward", FixtureKind::RewardHacking, "reward-hack-detected"),
    ];
    for (id, kind, code) in cases {
        let lease = scheduler
            .schedule([spec(
                id,
                kind,
                ExpectedOutcome::causal_failure(code, 1),
                false,
            )])
            .unwrap()
            .pop()
            .unwrap();
        let result = scheduler.finish(&lease, ObservedOutcome::failure(code, 1));
        #[cfg(target_os = "freebsd")]
        assert!(matches!(result, Err(FixtureScheduleError::Integrity(_))));
        #[cfg(not(target_os = "freebsd"))]
        {
            assert_eq!(result.unwrap(), RunDisposition::CleanupFailure);
            let retained = scheduler.run(&lease).unwrap();
            assert_eq!(
                retained.lease.disposition(),
                &LeaseDisposition::RecoveryRequired
            );
            assert!(!retained.is_active());
        }
    }
    #[cfg(target_os = "freebsd")]
    assert_eq!(scheduler.active_count(), 0);
    #[cfg(not(target_os = "freebsd"))]
    assert_eq!(scheduler.active_count(), 4);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn flaky_quarantine_never_raises_a_claim_ceiling() {
    let root = root("flaky");
    let mut scheduler = FixtureScheduler::new(&root);
    let id = scheduler
        .schedule([spec(
            "flaky",
            FixtureKind::Positive,
            ExpectedOutcome::pass(4),
            true,
        )])
        .unwrap()
        .pop()
        .unwrap();
    let result = scheduler.finish(&id, ObservedOutcome::pass(255));
    #[cfg(target_os = "freebsd")]
    assert!(matches!(result, Err(FixtureScheduleError::Integrity(_))));
    #[cfg(not(target_os = "freebsd"))]
    {
        assert_eq!(result.unwrap(), RunDisposition::CleanupFailure);
        let retained = scheduler.run(&id).unwrap();
        assert_eq!(
            retained.lease.disposition(),
            &LeaseDisposition::RecoveryRequired
        );
        assert!(!retained.is_active());
        assert_eq!(scheduler.active_count(), 1);
    }
    assert_eq!(ObservedOutcome::quarantined("flaky").claim_ceiling, 0);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn partial_schedule_acquisition_rolls_back_and_retains_recoverable_leases() {
    let root = root("partial-acquisition");
    let first = spec("alpha", FixtureKind::Positive, ExpectedOutcome::pass(3), false);
    let second = spec("beta", FixtureKind::Positive, ExpectedOutcome::pass(3), false);
    let expected_first_id = format!(
        "{}-{}",
        first.id,
        stable_digest(&format!("{}:1", first.metadata_digest))
    );
    let collision = root.join(format!(
        "{}-{}",
        second.id,
        stable_digest(&format!("{}:2", second.metadata_digest))
    ));
    fs::create_dir_all(&collision).unwrap();
    let mut scheduler = FixtureScheduler::new(&root);
    let error = scheduler.schedule([first, second]).unwrap_err();
    let recovery_lease_ids = match error {
        FixtureScheduleError::ScheduleRollback {
            source,
            recovery_lease_ids,
        } => {
            assert!(matches!(*source, FixtureScheduleError::Cleanup { .. }));
            recovery_lease_ids
        }
        other => panic!("expected typed schedule rollback, observed {other:?}"),
    };
    #[cfg(target_os = "freebsd")]
    {
        let collision_id = collision.file_name().unwrap().to_str().unwrap();
        assert_eq!(recovery_lease_ids, vec![collision_id.to_owned()]);
        assert_eq!(scheduler.active_count(), 1);
        assert_eq!(scheduler.run(collision_id).unwrap().lease.disposition(), &LeaseDisposition::RecoveryRequired);
    }
    #[cfg(not(target_os = "freebsd"))]
    {
        assert_eq!(recovery_lease_ids, vec![collision.file_name().unwrap().to_str().unwrap().to_owned(), expected_first_id.clone()]);
        let run = scheduler.run(&expected_first_id).unwrap();
        assert_eq!(run.lease.disposition(), &LeaseDisposition::RecoveryRequired);
        assert!(!run.is_active());
    }
    fs::remove_dir_all(root).unwrap();
}
#[cfg(unix)]
#[test]
fn acquisition_custody_matrix_records_every_stage_and_retains_unpinned_retries() {
    use std::cell::RefCell;
    use std::rc::Rc;
    let root = root("acquisition-custody-matrix");
    let stages = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&stages);
    set_lease_acquisition_hook(Some(Box::new(move |stage, _| observed.borrow_mut().push(stage))));
    let all_resources = FixtureSpec::new(
        "matrix",
        FixtureKind::Positive,
        "acquisition-matrix",
        BTreeSet::from([ResourceKind::File, ResourceKind::Port]),
        ExpectedOutcome::pass(3),
        false,
    )
    .unwrap();
    let mut scheduler = FixtureScheduler::new(&root);
    let id = scheduler.schedule([all_resources]).unwrap().pop().unwrap();
    set_lease_acquisition_hook(None);
    for stage in [
        LeaseAcquisitionStage::Pin,
        LeaseAcquisitionStage::Marker,
        LeaseAcquisitionStage::ResourceDirectory,
        LeaseAcquisitionStage::PortBind,
        LeaseAcquisitionStage::PortAddress,
        LeaseAcquisitionStage::ReservationWrite,
        LeaseAcquisitionStage::BeforeInsertion,
    ] {
        assert!(stages.borrow().contains(&stage), "missing {stage:?}");
    }
    assert!(scheduler.run(&id).unwrap().is_active());
    let retry_spec = spec("retry", FixtureKind::Positive, ExpectedOutcome::pass(3), false);
    let retry_id = format!("{}-{}", retry_spec.id, stable_digest(&format!("{}:1", retry_spec.metadata_digest)));
    let retry_root = root.join(&retry_id);
    fs::create_dir_all(&retry_root).unwrap();
    for _ in 0..2 {
        let mut retried = FixtureScheduler::new(&root);
        let error = retried.schedule([retry_spec.clone()]).unwrap_err();
        assert!(matches!(error, FixtureScheduleError::ScheduleRollback { ref recovery_lease_ids, .. } if recovery_lease_ids == &vec![retry_id.clone()]));
        let retained = retried.run(&retry_id).unwrap();
        assert_eq!(retained.lease.disposition(), &LeaseDisposition::RecoveryRequired);
        assert!(retry_root.exists());
        assert_eq!(retried.recover(&retry_id).unwrap(), RunDisposition::CleanupFailure);
        assert!(retry_root.exists());
    }
    assert!(scheduler.run(&id).unwrap().is_active());
    drop(scheduler);
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)] #[test]
fn retained_collision_is_recovery_only_and_cannot_launch() {
    struct Never; impl FixtureExecutor for Never { fn execute(&self, _: &FixtureSpec, _: &IsolationLease, _: &std::collections::BTreeMap<String, String>) -> Result<ObservedOutcome, FixtureScheduleError> { panic!("recovery lease reached executor") } }
    let root = root("collision-recovery-only"); let spec = spec("collision", FixtureKind::Positive, ExpectedOutcome::pass(3), false);
    let id = format!("{}-{}", spec.id, stable_digest(&format!("{}:1", spec.metadata_digest))); let retained = root.join(&id); fs::create_dir_all(&retained).unwrap(); fs::write(retained.join("sentinel"), b"unchanged").unwrap();
    let mut scheduler = FixtureScheduler::new(&root); assert!(matches!(scheduler.schedule([spec]), Err(FixtureScheduleError::ScheduleRollback { .. })));
    assert!(matches!(scheduler.execute(&id, &Never), Err(FixtureScheduleError::Integrity(message)) if message == "fixture execution requires an active lease"));
    assert_eq!(fs::read(retained.join("sentinel")).unwrap(), b"unchanged"); assert_eq!(scheduler.run(&id).unwrap().lease.disposition(), &LeaseDisposition::RecoveryRequired); fs::remove_dir_all(root).unwrap();
}
