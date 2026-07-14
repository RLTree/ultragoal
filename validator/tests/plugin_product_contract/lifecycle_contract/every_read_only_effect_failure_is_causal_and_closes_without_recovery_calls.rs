#[test]
fn every_read_only_effect_failure_is_causal_and_closes_without_recovery_calls() {
    for intent in [
        LifecycleIntent::RepeatUse,
        LifecycleIntent::IdempotentReinstall,
    ] {
        for fail_at in [
            LifecycleEffect::VerifyInstalledBytes,
            LifecycleEffect::ProbeRuntime,
        ] {
            let (state, read_only) = read_only_plan(intent);
            let replay = read_only.clone();
            let mut adapter = Adapter {
                fail_at: Some(fail_at),
                ..Adapter::default()
            };
            assert_eq!(
                apply(&state, &read_only, &mut adapter),
                Err(LifecycleError::ReadEffectFailed {
                    effect: fail_at,
                    causal_error: format!("injected-{fail_at:?}"),
                })
            );
            let failed_index = read_only
                .effects
                .iter()
                .position(|effect| *effect == fail_at)
                .unwrap();
            assert_eq!(adapter.effects, read_only.effects[..=failed_index]);
            assert!(adapter.restored.is_empty());
            assert_eq!(adapter.observe_calls.get(), 0);
            assert_eq!(
                recovery_token(&read_only),
                Err(LifecycleError::InvalidTransition)
            );

            let mut replay_adapter = Adapter::default();
            assert_eq!(
                apply(&state, &replay, &mut replay_adapter),
                Err(LifecycleError::ReplayedPlan)
            );
            assert!(replay_adapter.effects.is_empty());
            assert!(replay_adapter.restored.is_empty());
            assert_eq!(replay_adapter.observe_calls.get(), 0);
        }
    }
}

#[test]
fn concurrent_read_only_failure_has_one_causal_path_and_no_restore_path() {
    let (state, read_only) = read_only_plan(LifecycleIntent::RepeatUse);
    let barrier = Arc::new(Barrier::new(2));
    let handles = (0..2)
        .map(|_| {
            let state = state.clone();
            let read_only = read_only.clone();
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                let mut adapter = Adapter {
                    fail_at: Some(LifecycleEffect::VerifyInstalledBytes),
                    ..Adapter::default()
                };
                barrier.wait();
                let result = apply(&state, &read_only, &mut adapter);
                (
                    result,
                    adapter.effects.len(),
                    adapter.restored.len(),
                    adapter.observe_calls.get(),
                )
            })
        })
        .collect::<Vec<_>>();
    let results = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        results
            .iter()
            .filter(|(result, _, _, _)| {
                *result
                    == Err(LifecycleError::ReadEffectFailed {
                        effect: LifecycleEffect::VerifyInstalledBytes,
                        causal_error: "injected-VerifyInstalledBytes".to_owned(),
                    })
            })
            .count(),
        1
    );
    assert_eq!(
        results
            .iter()
            .filter(|(result, _, _, _)| *result == Err(LifecycleError::ReplayedPlan))
            .count(),
        1
    );
    assert_eq!(
        results
            .iter()
            .map(|(_, effect_count, _, _)| effect_count)
            .sum::<usize>(),
        1
    );
    assert!(results.iter().all(|(_, _, restore_count, observe_count)| {
        *restore_count == 0 && *observe_count == 0
    }));
}

#[test]
fn serialized_mutated_read_only_plan_refuses_before_every_adapter_call() {
    let (state, read_only) = read_only_plan(LifecycleIntent::IdempotentReinstall);
    let mut transported: LifecyclePlan =
        serde_json::from_slice(&serde_json::to_vec(&read_only).unwrap()).unwrap();
    transported.effects.insert(0, LifecycleEffect::RemoveCache);
    transported.writes_host_state = true;
    recompute_public_plan_id(&mut transported);
    let mut adapter = Adapter {
        fail_at: Some(LifecycleEffect::VerifyInstalledBytes),
        ..Adapter::default()
    };
    assert_eq!(
        apply(&state, &transported, &mut adapter),
        Err(LifecycleError::UnsealedPlan)
    );
    assert!(adapter.effects.is_empty());
    assert!(adapter.restored.is_empty());
    assert_eq!(adapter.observe_calls.get(), 0);
}

#[test]
fn repeat_use_and_reinstall_failures_preserve_the_recursive_tree_exactly() {
    let root = ZeroWriteRoot::new();
    let baseline = recursive_snapshot(root.path());
    for intent in [
        LifecycleIntent::RepeatUse,
        LifecycleIntent::IdempotentReinstall,
    ] {
        for fail_at in [
            LifecycleEffect::VerifyInstalledBytes,
            LifecycleEffect::ProbeRuntime,
        ] {
            let (state, read_only) = read_only_plan(intent);
            let before = recursive_snapshot(root.path());
            let mut adapter = ZeroWriteTrapAdapter::new(root.path(), fail_at);
            assert_eq!(
                apply(&state, &read_only, &mut adapter),
                Err(LifecycleError::ReadEffectFailed {
                    effect: fail_at,
                    causal_error: format!("read-failed-{fail_at:?}"),
                })
            );
            assert_eq!(adapter.restore_calls, 0);
            assert_eq!(adapter.observe_calls.get(), 0);
            assert_eq!(recursive_snapshot(root.path()), before);
        }
    }
    assert_eq!(recursive_snapshot(root.path()), baseline);
}

#[test]
fn stale_plan_race_is_rejected_before_any_effect() {
    let current = authority("0.0.12", D1);
    let before = installed(current.clone());
    let update = plan(
        &before,
        &request(
            LifecycleIntent::MonotonicUpdate,
            Some(authority("0.0.13", D2)),
            auth(Some(&current)),
        ),
    )
    .unwrap();
    let mut drifted = before;
    drifted.generation += 1;
    let mut adapter = Adapter::default();
    assert_eq!(
        apply(&drifted, &update, &mut adapter),
        Err(LifecycleError::StalePlan)
    );
    assert!(adapter.effects.is_empty());
}
