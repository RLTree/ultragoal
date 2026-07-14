#[test]
fn plan_identity_binds_authorization_and_rejects_post_plan_mutation() {
    let current = authority("0.0.12", D1);
    let before = installed(current.clone());
    let mut update = plan(
        &before,
        &request(
            LifecycleIntent::MonotonicUpdate,
            Some(authority("0.0.13", D2)),
            auth(Some(&current)),
        ),
    )
    .unwrap();
    let original_id = update.plan_id.clone();
    update.expected_after.generation += 1;
    let mut adapter = Adapter::default();
    assert_eq!(
        apply(&before, &update, &mut adapter),
        Err(LifecycleError::InvalidTransition)
    );
    assert!(adapter.effects.is_empty());
    assert_eq!(original_id.len(), "sha256:".len() + 64);
}

#[test]
fn successful_apply_arms_exactly_one_explicit_recovery() {
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
    let token = recovery_token(&update).unwrap();
    let mut apply_adapter = Adapter::default();
    let report = apply(&before, &update, &mut apply_adapter).unwrap();
    assert_eq!(report.state, update.expected_after);
    let mut adapter = Adapter::default();
    assert_eq!(
        recover(&report.state, &token, &mut adapter).unwrap(),
        before
    );
    assert_eq!(adapter.restored, vec![before]);
}

#[test]
fn failed_effect_and_failed_automatic_restore_preserve_one_recovery_action() {
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
    let token = recovery_token(&update).unwrap();
    let mut failing = PartialFailureAdapter::new(before.clone(), LifecycleEffect::RefreshCache);
    assert_eq!(
        apply(&before, &update, &mut failing),
        Err(LifecycleError::RecoveryFailed(
            "injected-restore".to_owned()
        ))
    );
    assert_eq!(failing.restored, vec![before.clone()]);
    assert_eq!(
        failing.completed_effects,
        vec![LifecycleEffect::InstallPackage]
    );
    assert_eq!(
        failing.attempted_effects,
        vec![
            LifecycleEffect::InstallPackage,
            LifecycleEffect::RefreshCache,
        ]
    );
    let partial = failing.observe_state().unwrap();
    assert_ne!(partial, update.expected_after);
    assert_eq!(partial.installed, update.expected_after.installed);
    assert_eq!(partial.cache, before.cache);
    assert_eq!(partial.generation, update.expected_after.generation);
    assert!(partial.recovery_required);

    let mut planned_final = Adapter::default();
    assert_eq!(
        recover(&update.expected_after, &token, &mut planned_final),
        Err(LifecycleError::StaleRecoveryToken)
    );
    assert!(planned_final.restored.is_empty());

    let mut newer_generation = partial.clone();
    newer_generation.generation += 1;
    let mut newer_adapter = Adapter::default();
    assert_eq!(
        recover(&newer_generation, &token, &mut newer_adapter),
        Err(LifecycleError::StaleRecoveryToken)
    );
    assert!(newer_adapter.restored.is_empty());

    let mut substituted_candidate = partial.clone();
    substituted_candidate
        .installed
        .as_mut()
        .unwrap()
        .candidate_id = D2.to_owned();
    let mut substituted_adapter = Adapter::default();
    assert_eq!(
        recover(&substituted_candidate, &token, &mut substituted_adapter),
        Err(LifecycleError::StaleRecoveryToken)
    );
    assert!(substituted_adapter.restored.is_empty());

    let mut recovery = Adapter::default();
    assert_eq!(recover(&partial, &token, &mut recovery).unwrap(), before);
    assert_eq!(recovery.restored, vec![before]);
}

#[test]
fn unmodeled_failed_restore_observation_closes_recovery_authority() {
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
    let token = recovery_token(&update).unwrap();
    let mut failing = PartialFailureAdapter::new(before.clone(), LifecycleEffect::RefreshCache);
    failing.substitute_observed_candidate = true;
    assert_eq!(
        apply(&before, &update, &mut failing),
        Err(LifecycleError::RecoveryStateMismatch)
    );
    let legitimate_partial = failing.current.clone();
    let mut refused = Adapter::default();
    assert_eq!(
        recover(&legitimate_partial, &token, &mut refused),
        Err(LifecycleError::ReplayedRecoveryToken)
    );
    assert!(refused.restored.is_empty());
}

#[test]
fn host_mutations_fail_closed_without_authorization() {
    let target = authority("0.0.12", D1);
    assert_eq!(
        plan(
            &LifecycleState::default(),
            &request(
                LifecycleIntent::FreshInstall,
                Some(target),
                LifecycleAuthorization {
                    allow_host_write: false,
                    allow_downgrade: false,
                    expected_installed_sha256: None,
                },
            )
        ),
        Err(LifecycleError::AuthorizationRequired)
    );
}
