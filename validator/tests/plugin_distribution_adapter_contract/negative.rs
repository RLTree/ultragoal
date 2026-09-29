use crate::lifecycle_fixture::{
    CANDIDATE, Fixture, OTHER_CANDIDATE, installed, lifecycle, request,
};
use crate::plugin_product::distribution_adapter::{AdapterErrorId, DistributionLifecycleOperation};
use crate::plugin_product::lifecycle::{
    LifecycleEffect, LifecycleError, LifecycleIntent, LifecycleState,
};

#[test]
fn package_plan_snapshot_and_lifecycle_substitutions_refuse_at_bind() {
    let fixture = Fixture::new("binding-substitution");
    let v11 = fixture.bundle("0.0.11");
    let v12 = fixture.bundle("0.0.12");
    let empty = LifecycleState::default();
    let fresh = lifecycle(
        &empty,
        request(
            LifecycleIntent::FreshInstall,
            Some(v11.authority.clone()),
            None,
            None,
            true,
            false,
        ),
    );
    let before = fixture.tree();
    let failure =
        DistributionLifecycleOperation::bind(fixture.confined(), &v11.plan, &v12.snapshot, &fresh)
            .err()
            .unwrap();
    assert_eq!(failure.id(), AdapterErrorId::InvalidPackageBinding);
    assert_eq!(fixture.tree(), before);

    for dimension in ["package", "inventory", "candidate", "version"] {
        let mut authority = v11.authority.clone();
        match dimension {
            "package" => authority.package_sha256 = OTHER_CANDIDATE.into(),
            "inventory" => authority.inventory_sha256 = OTHER_CANDIDATE.into(),
            "candidate" => authority.candidate_id = OTHER_CANDIDATE.into(),
            "version" => authority.version.patch += 1,
            _ => unreachable!(),
        }
        let altered = lifecycle(
            &empty,
            request(
                LifecycleIntent::FreshInstall,
                Some(authority.clone()),
                None,
                None,
                true,
                false,
            ),
        );
        let failure = DistributionLifecycleOperation::bind(
            fixture.confined(),
            &v11.plan,
            &v11.snapshot,
            &altered,
        )
        .err()
        .unwrap();
        assert_eq!(failure.id(), AdapterErrorId::InvalidLifecycleBinding);
    }
    assert_eq!(v11.snapshot.candidate_id(), CANDIDATE);

    let mut transported = fresh.clone();
    transported.effects.insert(0, LifecycleEffect::RemoveCache);
    let failure = DistributionLifecycleOperation::bind(
        fixture.confined(),
        &v11.plan,
        &v11.snapshot,
        &transported,
    )
    .err()
    .unwrap();
    assert_eq!(failure.id(), AdapterErrorId::InvalidLifecycleBinding);
    let mut wrong_intent = fresh.clone();
    wrong_intent.intent = LifecycleIntent::UninstallTeardown;
    let failure = DistributionLifecycleOperation::bind(
        fixture.confined(),
        &v11.plan,
        &v11.snapshot,
        &wrong_intent,
    )
    .err()
    .unwrap();
    assert_eq!(failure.id(), AdapterErrorId::InvalidLifecycleBinding);
    assert_eq!(fixture.tree(), before);
}

#[test]
fn verify_and_probe_failures_are_both_read_only_and_never_restore() {
    for (surface, expected_effect) in [
        ("installed", LifecycleEffect::VerifyInstalledBytes),
        ("cache", LifecycleEffect::ProbeRuntime),
    ] {
        let fixture = Fixture::new(&format!("read-only-{surface}-failure"));
        let v11 = fixture.bundle("0.0.11");
        let v12 = fixture.bundle("0.0.12");
        let state = installed(&v11.authority, 4);
        for path in [
            "installed/harness-ultragoal.hugpkg",
            "cache/harness-ultragoal.hugpkg",
        ] {
            fixture.replace(path, None, Some(v11.snapshot.archive()));
        }
        let repeat = lifecycle(
            &state,
            request(
                LifecycleIntent::RepeatUse,
                Some(v11.authority.clone()),
                None,
                state.installed.as_ref(),
                false,
                false,
            ),
        );
        let mut operation = fixture.operation(&v11, &repeat);
        fixture.replace(
            &format!("{surface}/harness-ultragoal.hugpkg"),
            Some(&v11.authority.package_sha256),
            Some(v12.snapshot.archive()),
        );
        let before = fixture.tree();
        let failure = operation.apply(&state, &repeat).unwrap_err();
        assert!(matches!(
            failure,
            LifecycleError::ReadEffectFailed { effect, .. } if effect == expected_effect
        ));
        assert_eq!(fixture.tree(), before);
    }
}

#[test]
fn independently_issued_identical_plan_is_rejected_before_effects() {
    let fixture = Fixture::new("independent-identical-plan");
    let v11 = fixture.bundle("0.0.11");
    let empty = LifecycleState::default();
    let issue = || {
        lifecycle(
            &empty,
            request(
                LifecycleIntent::FreshInstall,
                Some(v11.authority.clone()),
                None,
                None,
                true,
                false,
            ),
        )
    };
    let bound = issue();
    let sibling = issue();
    assert_eq!(bound.plan_id, sibling.plan_id);
    assert_ne!(bound, sibling);

    let mut operation = fixture.operation(&v11, &bound);
    let before = fixture.tree();
    assert_eq!(
        operation.apply(&empty, &sibling),
        Err(LifecycleError::InvalidTransition)
    );
    assert_eq!(fixture.tree(), before);

    let applied = operation.apply(&empty, &bound).unwrap().state;
    let token = operation.recovery_token().unwrap();
    assert_eq!(operation.recover(&applied, &token).unwrap(), empty);
}
