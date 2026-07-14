#[test]
fn partial_cache_failure_rolls_back_only_the_completed_install_prefix() {
    let fixture = Fixture::new("partial-prefix");
    let v11 = fixture.bundle("0.0.11");
    fs::create_dir(fixture.root.join("cache")).unwrap();
    fs::set_permissions(
        fixture.root.join("cache"),
        fs::Permissions::from_mode(0o500),
    )
    .unwrap();
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
    let mut operation = fixture.operation(&v11, &fresh);
    let report = operation.apply(&empty, &fresh).unwrap();
    fs::set_permissions(
        fixture.root.join("cache"),
        fs::Permissions::from_mode(0o700),
    )
    .unwrap();
    assert_eq!(report.disposition, ApplyDisposition::RecoveredAfterFailure);
    assert_eq!(report.state, empty);
    assert_eq!(report.completed_effects, vec![fresh.effects[0]]);
    assert_eq!(operation.observed_mutation_count(), 0);
    assert_eq!(operation.observe_state().unwrap(), empty);
}

#[test]
fn successful_update_recovery_token_restores_exact_prior_once() {
    let fixture = Fixture::new("successful-recovery");
    let v11 = fixture.bundle("0.0.11");
    let v12 = fixture.bundle("0.0.12");
    let before = installed(&v11.authority, 8);
    fixture.replace(
        "installed/harness-ultragoal.hugpkg",
        None,
        Some(v11.snapshot.archive()),
    );
    fixture.replace(
        "cache/harness-ultragoal.hugpkg",
        None,
        Some(v11.snapshot.archive()),
    );
    let update = lifecycle(
        &before,
        request(
            LifecycleIntent::MonotonicUpdate,
            Some(v12.authority.clone()),
            None,
            before.installed.as_ref(),
            true,
            false,
        ),
    );
    let mut operation = fixture.operation(&v12, &update);
    let after = operation.apply(&before, &update).unwrap().state;
    let sibling_token = issue_sibling_recovery_token(&update).unwrap();
    let before_sibling_refusal = fixture.tree();
    assert_eq!(
        operation.recover(&after, &sibling_token),
        Err(LifecycleError::InvalidTransition)
    );
    assert_eq!(fixture.tree(), before_sibling_refusal);

    let token = operation.recovery_token().unwrap();
    assert_eq!(operation.observed_mutation_count(), 2);
    let mut substituted = token.clone();
    substituted
        .expected_current
        .installed
        .as_mut()
        .unwrap()
        .candidate_id = crate::lifecycle_fixture::OTHER_CANDIDATE.to_owned();
    let before_refusal = fixture.tree();
    assert_eq!(
        operation.recover(&after, &substituted),
        Err(LifecycleError::InvalidTransition)
    );
    assert_eq!(fixture.tree(), before_refusal);
    assert_eq!(operation.recover(&after, &token).unwrap(), before);
    assert_eq!(operation.observe_state().unwrap(), before);
    assert_eq!(
        operation.recover(&before, &token),
        Err(LifecycleError::ReplayedRecoveryToken)
    );
}

#[test]
fn independently_applied_cross_root_token_is_rejected_before_restore() {
    let left_fixture = Fixture::new("cross-root-left");
    let right_fixture = Fixture::new("cross-root-right");
    let left_v11 = left_fixture.bundle("0.0.11");
    let left_v12 = left_fixture.bundle("0.0.12");
    let right_v11 = right_fixture.bundle("0.0.11");
    let right_v12 = right_fixture.bundle("0.0.12");
    assert_eq!(left_v11.authority, right_v11.authority);
    assert_eq!(left_v12.authority, right_v12.authority);

    let before = installed(&left_v11.authority, 8);
    for (fixture, bundle) in [(&left_fixture, &left_v11), (&right_fixture, &right_v11)] {
        fixture.replace(
            "installed/harness-ultragoal.hugpkg",
            None,
            Some(bundle.snapshot.archive()),
        );
        fixture.replace(
            "cache/harness-ultragoal.hugpkg",
            None,
            Some(bundle.snapshot.archive()),
        );
    }
    let issue = |target: &crate::lifecycle_fixture::Bundle| {
        lifecycle(
            &before,
            request(
                LifecycleIntent::MonotonicUpdate,
                Some(target.authority.clone()),
                None,
                before.installed.as_ref(),
                true,
                false,
            ),
        )
    };
    let left_plan = issue(&left_v12);
    let right_plan = issue(&right_v12);
    assert_eq!(left_plan.plan_id, right_plan.plan_id);
    assert_ne!(left_plan, right_plan);

    let mut left = left_fixture.operation(&left_v12, &left_plan);
    let mut right = right_fixture.operation(&right_v12, &right_plan);
    let left_after = left.apply(&before, &left_plan).unwrap().state;
    let right_after = right.apply(&before, &right_plan).unwrap().state;
    let left_token = left.recovery_token().unwrap();
    let right_token = right.recovery_token().unwrap();
    let left_tree = left_fixture.tree();
    let right_tree = right_fixture.tree();

    assert_eq!(
        left.recover(&left_after, &right_token),
        Err(LifecycleError::InvalidTransition)
    );
    assert_eq!(left.observed_mutation_count(), 2);
    assert_eq!(right.observed_mutation_count(), 2);
    assert_eq!(left_fixture.tree(), left_tree);
    assert_eq!(right_fixture.tree(), right_tree);

    assert_eq!(left.recover(&left_after, &left_token).unwrap(), before);
    assert_eq!(right.recover(&right_after, &right_token).unwrap(), before);
}
