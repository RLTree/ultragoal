#[test]
fn same_root_sibling_operation_cannot_issue_or_use_owner_token() {
    let fixture = Fixture::new("same-root-sibling");
    let v11 = fixture.bundle("0.0.11");
    let empty = LifecycleState::default();
    let plan = lifecycle(
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
    let mut owner = fixture.operation(&v11, &plan);
    let mut sibling = fixture.operation(&v11, &plan);
    let after = owner.apply(&empty, &plan).unwrap().state;
    assert_eq!(
        sibling.apply(&empty, &plan),
        Err(LifecycleError::ReplayedPlan)
    );
    assert_eq!(
        sibling.recovery_token(),
        Err(LifecycleError::RecoveryUnavailable)
    );

    let token = owner.recovery_token().unwrap();
    let before_refusal = fixture.tree();
    assert_eq!(
        sibling.recover(&after, &token),
        Err(LifecycleError::InvalidTransition)
    );
    assert_eq!(fixture.tree(), before_refusal);
    assert_eq!(owner.recover(&after, &token).unwrap(), empty);
}

#[test]
fn concurrent_recovery_token_replay_restores_exactly_once() {
    let fixture = Fixture::new("concurrent-token-replay");
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
    let token = operation.recovery_token().unwrap();
    let operation = Arc::new(Mutex::new(operation));
    let start = Arc::new(Barrier::new(3));
    let run = || {
        let operation = Arc::clone(&operation);
        let start = Arc::clone(&start);
        let after = after.clone();
        let token = token.clone();
        std::thread::spawn(move || {
            start.wait();
            operation.lock().unwrap().recover(&after, &token)
        })
    };
    let left = run();
    let right = run();
    start.wait();
    let outcomes = [left.join().unwrap(), right.join().unwrap()];
    assert_eq!(outcomes.iter().filter(|row| row.is_ok()).count(), 1);
    assert_eq!(
        outcomes
            .iter()
            .filter(|row| matches!(row, Err(LifecycleError::ReplayedRecoveryToken)))
            .count(),
        1
    );
    assert_eq!(operation.lock().unwrap().observe_state().unwrap(), before);
}

#[test]
fn concurrent_replay_of_one_sealed_plan_authorizes_exactly_one_transition() {
    let fixture = Fixture::new("concurrent-replay");
    let v11 = fixture.bundle("0.0.11");
    let empty = LifecycleState::default();
    let plan = lifecycle(
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
    let first = fixture.operation(&v11, &plan);
    let second = fixture.operation(&v11, &plan);
    let start = Arc::new(Barrier::new(3));
    let run = |mut operation: crate::plugin_product::distribution_adapter::DistributionLifecycleOperation| {
        let start = Arc::clone(&start);
        let plan = plan.clone();
        let empty = empty.clone();
        std::thread::spawn(move || {
            start.wait();
            operation.apply(&empty, &plan)
        })
    };
    let left = run(first);
    let right = run(second);
    start.wait();
    let outcomes = [left.join().unwrap(), right.join().unwrap()];
    assert_eq!(outcomes.iter().filter(|row| row.is_ok()).count(), 1);
    assert_eq!(
        outcomes
            .iter()
            .filter(|row| matches!(row, Err(LifecycleError::ReplayedPlan)))
            .count(),
        1
    );
}
