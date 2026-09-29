#[test]
fn explicit_failed_update_recovery_is_prior_bound() {
    let prior = installed(authority("0.0.12", D1));
    let interrupted = LifecycleState {
        installed: Some(authority("0.0.13", D2)),
        cache: prior.cache.clone(),
        generation: 8,
        recovery_required: true,
    };
    let recovery = LifecycleRequest {
        intent: LifecycleIntent::FailedUpdateRecovery,
        target: None,
        prior_authority: Some(prior.clone()),
        authorization: LifecycleAuthorization {
            allow_host_write: true,
            allow_downgrade: true,
            expected_installed_sha256: Some(D2.to_owned()),
        },
    };
    let plan = plan(&interrupted, &recovery).unwrap();
    assert_eq!(plan.expected_after.installed, prior.installed);
    assert!(!plan.expected_after.recovery_required);
    let mut adapter = Adapter::default();
    let report = apply(&interrupted, &plan, &mut adapter).unwrap();
    verify(&report.state, &plan).unwrap();
}

#[test]
fn failed_update_recovery_refuses_a_normal_state_before_effects_are_planned() {
    let prior = installed(authority("0.0.12", D1));
    let normal = installed(authority("0.0.13", D2));
    let recovery = LifecycleRequest {
        intent: LifecycleIntent::FailedUpdateRecovery,
        target: None,
        prior_authority: Some(prior),
        authorization: LifecycleAuthorization {
            allow_host_write: true,
            allow_downgrade: true,
            expected_installed_sha256: Some(D2.to_owned()),
        },
    };

    assert_eq!(
        plan(&normal, &recovery),
        Err(LifecycleError::InvalidTransition)
    );
}

#[test]
fn downgrade_requires_specific_authority_and_expected_prior() {
    let current = authority("0.0.13", D2);
    let before = installed(current.clone());
    let mut authorization = auth(Some(&current));
    let rollback_request = request(
        LifecycleIntent::AuthorizedRollback,
        Some(authority("0.0.12", D1)),
        authorization.clone(),
    );
    assert_eq!(
        plan(&before, &rollback_request),
        Err(LifecycleError::DowngradeAuthorizationRequired)
    );
    authorization.allow_downgrade = true;
    let rollback = plan(
        &before,
        &request(
            LifecycleIntent::AuthorizedRollback,
            Some(authority("0.0.12", D1)),
            authorization,
        ),
    )
    .unwrap();
    assert_eq!(
        rollback.expected_after.installed.unwrap().package_sha256,
        D1
    );
}

#[test]
fn reinstall_is_idempotent_and_read_only_but_rejects_mismatch() {
    let current = authority("0.0.12", D1);
    let before = installed(current.clone());
    let reinstall = plan(
        &before,
        &request(
            LifecycleIntent::IdempotentReinstall,
            Some(current.clone()),
            LifecycleAuthorization {
                allow_host_write: false,
                allow_downgrade: false,
                expected_installed_sha256: Some(D1.to_owned()),
            },
        ),
    )
    .unwrap();
    assert!(!reinstall.writes_host_state);
    assert_eq!(reinstall.expected_after, before);
    assert_eq!(
        plan(
            &before,
            &request(
                LifecycleIntent::IdempotentReinstall,
                Some(authority("0.0.12", D2)),
                auth(Some(&current)),
            )
        ),
        Err(LifecycleError::InvalidTransition)
    );
}

#[test]
fn uninstall_teardown_removes_installed_and_cache_authority() {
    let current = authority("0.0.12", D1);
    let before = installed(current.clone());
    let teardown = plan(
        &before,
        &request(
            LifecycleIntent::UninstallTeardown,
            None,
            auth(Some(&current)),
        ),
    )
    .unwrap();
    assert_eq!(teardown.expected_after.installed, None);
    assert_eq!(teardown.expected_after.cache, None);
    let mut adapter = Adapter::default();
    let report = apply(&before, &teardown, &mut adapter).unwrap();
    verify(&report.state, &teardown).unwrap();
    assert_eq!(
        adapter.effects.last(),
        Some(&LifecycleEffect::VerifyTeardown)
    );
}

#[test]
fn stale_cache_recovery_preserves_installed_authority() {
    let current = authority("0.0.13", D2);
    let before = LifecycleState {
        installed: Some(current.clone()),
        cache: Some(authority("0.0.12", D1)),
        generation: 9,
        recovery_required: false,
    };
    let recovery = plan(
        &before,
        &request(
            LifecycleIntent::StaleCacheRecovery,
            None,
            auth(Some(&current)),
        ),
    )
    .unwrap();
    assert_eq!(recovery.expected_after.installed, Some(current.clone()));
    assert_eq!(recovery.expected_after.cache, Some(current));
}

#[test]
fn repeat_use_is_zero_write_and_rejects_candidate_substitution() {
    let current = authority("0.0.12", D1);
    let before = installed(current.clone());
    let reuse = plan(
        &before,
        &request(
            LifecycleIntent::RepeatUse,
            Some(current.clone()),
            LifecycleAuthorization {
                allow_host_write: false,
                allow_downgrade: false,
                expected_installed_sha256: Some(D1.to_owned()),
            },
        ),
    )
    .unwrap();
    assert!(!reuse.writes_host_state);
    assert_eq!(reuse.expected_after, before);
    assert_eq!(
        plan(
            &before,
            &request(
                LifecycleIntent::RepeatUse,
                Some(authority("0.0.12", D2)),
                auth(Some(&current)),
            )
        ),
        Err(LifecycleError::ExpectedPriorMismatch)
    );
}
