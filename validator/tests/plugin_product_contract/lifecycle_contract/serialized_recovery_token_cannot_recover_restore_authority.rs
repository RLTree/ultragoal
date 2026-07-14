#[test]
fn serialized_recovery_token_cannot_recover_restore_authority() {
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
    let authorized = recovery_token(&update).unwrap();
    let mut apply_adapter = Adapter::default();
    apply(&before, &update, &mut apply_adapter).unwrap();
    let transported: RecoveryToken =
        serde_json::from_slice(&serde_json::to_vec(&authorized).unwrap()).unwrap();
    let mut adapter = Adapter::default();
    assert_eq!(
        recover(&update.expected_after, &transported, &mut adapter),
        Err(LifecycleError::UnsealedRecoveryToken)
    );
    assert!(adapter.effects.is_empty());
    assert!(adapter.restored.is_empty());
}

#[test]
fn recovery_token_rejects_restore_substitution_before_adapter_effects() {
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
    let mut substituted = recovery_token(&update).unwrap();
    substituted.prior = installed(authority("0.0.11", D3));
    let mut adapter = Adapter::default();
    assert_eq!(
        recover(&update.expected_after, &substituted, &mut adapter),
        Err(LifecycleError::InvalidTransition)
    );
    assert!(adapter.effects.is_empty());
    assert!(adapter.restored.is_empty());
    let mut expected_current_substitution = recovery_token(&update).unwrap();
    expected_current_substitution.expected_current.generation += 1;
    let mut second_adapter = Adapter::default();
    assert_eq!(
        recover(
            &update.expected_after,
            &expected_current_substitution,
            &mut second_adapter
        ),
        Err(LifecycleError::InvalidTransition)
    );
    assert!(second_adapter.effects.is_empty());
    assert!(second_adapter.restored.is_empty());
}

#[test]
fn stale_recovery_authority_refuses_without_consuming_the_valid_restore() {
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
    apply(&before, &update, &mut apply_adapter).unwrap();
    let mut stale = update.expected_after.clone();
    stale.generation += 1;
    let mut stale_adapter = Adapter::default();
    assert_eq!(
        recover(&stale, &token, &mut stale_adapter),
        Err(LifecycleError::StaleRecoveryToken)
    );
    assert!(stale_adapter.restored.is_empty());
    let mut authorized_adapter = Adapter::default();
    assert_eq!(
        recover(&update.expected_after, &token, &mut authorized_adapter).unwrap(),
        before
    );
    assert_eq!(authorized_adapter.restored, vec![before]);
}

#[test]
fn recovery_before_apply_refuses_without_consuming_later_recovery() {
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
    let mut premature = Adapter::default();
    assert_eq!(
        recover(&update.expected_after, &token, &mut premature),
        Err(LifecycleError::RecoveryUnavailable)
    );
    assert!(premature.restored.is_empty());
    let mut apply_adapter = Adapter::default();
    let report = apply(&before, &update, &mut apply_adapter).unwrap();
    let mut recovery = Adapter::default();
    assert_eq!(
        recover(&report.state, &token, &mut recovery).unwrap(),
        before
    );
    assert_eq!(recovery.restored, vec![before]);
}

#[test]
fn duplicate_tokens_and_clones_share_exactly_one_post_apply_recovery() {
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
    let duplicate = recovery_token(&update).unwrap();
    let cloned = token.clone();
    let mut apply_adapter = Adapter::default();
    let report = apply(&before, &update, &mut apply_adapter).unwrap();
    let mut first = Adapter::default();
    recover(&report.state, &token, &mut first).unwrap();
    for replay in [&duplicate, &cloned] {
        let mut adapter = Adapter::default();
        assert_eq!(
            recover(&report.state, replay, &mut adapter),
            Err(LifecycleError::ReplayedRecoveryToken)
        );
        assert!(adapter.restored.is_empty());
    }
    let mut replayed_apply = Adapter::default();
    assert_eq!(
        apply(&before, &update, &mut replayed_apply),
        Err(LifecycleError::ReplayedPlan)
    );
    assert!(replayed_apply.effects.is_empty());
}

#[test]
fn successful_automatic_restore_closes_redundant_recovery() {
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
    let mut failing = Adapter {
        fail_at: Some(LifecycleEffect::RefreshCache),
        ..Adapter::default()
    };
    let report = apply(&before, &update, &mut failing).unwrap();
    assert_eq!(report.disposition, ApplyDisposition::RecoveredAfterFailure);
    assert_eq!(report.state, before);
    let mut redundant = Adapter::default();
    assert_eq!(
        recover(&update.expected_after, &token, &mut redundant),
        Err(LifecycleError::ReplayedRecoveryToken)
    );
    assert!(redundant.restored.is_empty());
}
