#[test]
fn serialized_plan_is_transport_only_and_cannot_recover_apply_authority() {
    let current = authority("0.0.12", D1);
    let before = installed(current.clone());
    let authorized = plan(
        &before,
        &request(
            LifecycleIntent::MonotonicUpdate,
            Some(authority("0.0.13", D2)),
            auth(Some(&current)),
        ),
    )
    .unwrap();
    let transported: LifecyclePlan =
        serde_json::from_slice(&serde_json::to_vec(&authorized).unwrap()).unwrap();
    let mut adapter = Adapter::default();
    assert_eq!(
        apply(&before, &transported, &mut adapter),
        Err(LifecycleError::UnsealedPlan)
    );
    assert!(adapter.effects.is_empty());
    assert!(adapter.restored.is_empty());
}

#[test]
fn effect_expansion_cannot_turn_a_read_only_plan_into_write_authority() {
    let current = authority("0.0.12", D1);
    let before = installed(current.clone());
    let mut forged = plan(
        &before,
        &request(
            LifecycleIntent::RepeatUse,
            Some(current),
            LifecycleAuthorization {
                allow_host_write: false,
                allow_downgrade: false,
                expected_installed_sha256: Some(D1.to_owned()),
            },
        ),
    )
    .unwrap();
    forged.effects.insert(0, LifecycleEffect::RemoveCache);
    forged.writes_host_state = true;
    recompute_public_plan_id(&mut forged);
    let mut adapter = Adapter::default();
    assert_eq!(
        apply(&before, &forged, &mut adapter),
        Err(LifecycleError::InvalidTransition)
    );
    assert!(adapter.effects.is_empty());
    assert!(adapter.restored.is_empty());
}

#[test]
fn caller_write_flag_cannot_override_effect_derived_classification() {
    let current = authority("0.0.12", D1);
    let before = installed(current.clone());
    let mut forged = plan(
        &before,
        &request(
            LifecycleIntent::MonotonicUpdate,
            Some(authority("0.0.13", D2)),
            auth(Some(&current)),
        ),
    )
    .unwrap();
    forged.writes_host_state = false;
    recompute_public_plan_id(&mut forged);
    let mut adapter = Adapter::default();
    assert_eq!(
        apply(&before, &forged, &mut adapter),
        Err(LifecycleError::InvalidTransition)
    );
    assert!(adapter.effects.is_empty());
    assert!(adapter.restored.is_empty());
}

#[test]
fn recomputed_digest_cannot_substitute_the_authorized_target() {
    let current = authority("0.0.12", D1);
    let before = installed(current.clone());
    let mut substituted = plan(
        &before,
        &request(
            LifecycleIntent::MonotonicUpdate,
            Some(authority("0.0.13", D2)),
            auth(Some(&current)),
        ),
    )
    .unwrap();
    let replacement = authority("0.0.13", D3);
    substituted.expected_after.installed = Some(replacement.clone());
    substituted.expected_after.cache = Some(replacement);
    recompute_public_plan_id(&mut substituted);
    let mut adapter = Adapter::default();
    assert_eq!(
        apply(&before, &substituted, &mut adapter),
        Err(LifecycleError::InvalidTransition)
    );
    assert!(adapter.effects.is_empty());
    assert!(adapter.restored.is_empty());
}

#[test]
fn applied_plan_and_every_clone_are_replay_protected_before_effects() {
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
    let replay = update.clone();
    let mut first = Adapter::default();
    apply(&before, &update, &mut first).unwrap();
    let mut second = Adapter::default();
    assert_eq!(
        apply(&before, &replay, &mut second),
        Err(LifecycleError::ReplayedPlan)
    );
    assert!(second.effects.is_empty());
    assert!(second.restored.is_empty());
}

#[test]
fn concurrent_plan_replay_race_executes_exactly_one_authorized_transition() {
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
    let expected_effect_count = update.effects.len();
    let barrier = Arc::new(Barrier::new(2));
    let handles = (0..2)
        .map(|_| {
            let barrier = Arc::clone(&barrier);
            let before = before.clone();
            let update = update.clone();
            std::thread::spawn(move || {
                let mut adapter = Adapter::default();
                barrier.wait();
                let result = apply(&before, &update, &mut adapter);
                (result, adapter.effects.len(), adapter.restored.len())
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
            .filter(|(result, _, _)| result.is_ok())
            .count(),
        1
    );
    assert_eq!(
        results
            .iter()
            .filter(|(result, _, _)| *result == Err(LifecycleError::ReplayedPlan))
            .count(),
        1
    );
    assert_eq!(
        results.iter().map(|(_, effects, _)| effects).sum::<usize>(),
        expected_effect_count
    );
    assert!(results.iter().all(|(_, _, restored)| *restored == 0));
}
