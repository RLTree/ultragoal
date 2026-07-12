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
