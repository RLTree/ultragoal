#[test]
fn accepted_lifecycle_operations_require_their_exact_state_transitions() {
    let current = Fixture::new('b');
    let replacement = Fixture::new('c');
    let state = |generation, package: Option<&PackageIdentity>, recovery_required| {
        AcceptedHostState::new(generation, package.cloned(), recovery_required).unwrap()
    };
    let exact = AcceptedReconciliationPolicy::ExactPostStateAndSeparateHostLayers;
    let rows = [
        (
            AcceptedLifecycleOperation::FreshInstall,
            state(2, None, false),
            state(3, Some(&current.package), false),
            state(2, None, false),
            AcceptedRollbackPolicy::RemoveOnlyNewTarget,
            exact,
        ),
        (
            AcceptedLifecycleOperation::MonotonicUpdate,
            state(3, Some(&current.package), false),
            state(4, Some(&replacement.package), false),
            state(3, Some(&current.package), false),
            AcceptedRollbackPolicy::RestoreExactPreState,
            exact,
        ),
        (
            AcceptedLifecycleOperation::FailedUpdateRecovery,
            state(4, Some(&current.package), true),
            state(5, Some(&current.package), false),
            state(4, Some(&current.package), true),
            AcceptedRollbackPolicy::ManualReconciliationOnly,
            exact,
        ),
        (
            AcceptedLifecycleOperation::IdempotentReinstall,
            state(6, Some(&current.package), false),
            state(6, Some(&current.package), false),
            state(6, Some(&current.package), false),
            AcceptedRollbackPolicy::RestoreExactPreState,
            exact,
        ),
        (
            AcceptedLifecycleOperation::UninstallTeardown,
            state(7, Some(&current.package), false),
            state(8, None, false),
            state(7, Some(&current.package), false),
            AcceptedRollbackPolicy::RemoveOnlyNewTarget,
            AcceptedReconciliationPolicy::ExactAbsenceAndSeparateHostLayers,
        ),
        (
            AcceptedLifecycleOperation::RepeatUse,
            state(8, Some(&current.package), false),
            state(8, Some(&current.package), false),
            state(8, Some(&current.package), false),
            AcceptedRollbackPolicy::RestoreExactPreState,
            exact,
        ),
    ];

    for (operation, before, expected_after, rollback, policy, reconciliation) in rows {
        let plan = AcceptedLifecyclePlan::new(
            operation,
            before,
            expected_after,
            rollback,
            policy,
            reconciliation,
        )
        .unwrap();
        assert_eq!(plan.operation().as_str(), operation.as_str());
        assert!(plan.plan_sha256().starts_with("sha256:"));
    }
    for operation in [
        AcceptedLifecycleOperation::AuthorizedRollback,
        AcceptedLifecycleOperation::StaleCacheRecovery,
    ] {
        assert!(
            AcceptedLifecyclePlan::new(
                operation,
                state(9, Some(&current.package), false),
                state(10, Some(&replacement.package), false),
                state(9, Some(&current.package), false),
                AcceptedRollbackPolicy::RestoreExactPreState,
                exact,
            )
            .is_err()
        );
    }
}

#[test]
fn lifecycle_operation_matrix_rejects_sibling_state_mutations() {
    let current = Fixture::new('d');
    let replacement = Fixture::new('e');
    let state = |generation, package: Option<&PackageIdentity>, recovery_required| {
        AcceptedHostState::new(generation, package.cloned(), recovery_required).unwrap()
    };
    let exact = AcceptedReconciliationPolicy::ExactPostStateAndSeparateHostLayers;
    let rows = [
        (
            AcceptedLifecycleOperation::FreshInstall,
            state(2, None, false),
            state(3, None, false),
            state(2, None, false),
            AcceptedRollbackPolicy::RemoveOnlyNewTarget,
            exact,
        ),
        (
            AcceptedLifecycleOperation::MonotonicUpdate,
            state(3, Some(&current.package), false),
            state(3, Some(&replacement.package), false),
            state(3, Some(&current.package), false),
            AcceptedRollbackPolicy::RestoreExactPreState,
            exact,
        ),
        (
            AcceptedLifecycleOperation::FailedUpdateRecovery,
            state(4, Some(&current.package), false),
            state(5, Some(&current.package), false),
            state(4, Some(&current.package), false),
            AcceptedRollbackPolicy::ManualReconciliationOnly,
            exact,
        ),
        (
            AcceptedLifecycleOperation::AuthorizedRollback,
            state(5, Some(&replacement.package), false),
            state(6, Some(&current.package), false),
            state(5, Some(&replacement.package), false),
            AcceptedRollbackPolicy::RestoreExactPreState,
            exact,
        ),
        (
            AcceptedLifecycleOperation::IdempotentReinstall,
            state(6, Some(&current.package), false),
            state(6, Some(&replacement.package), false),
            state(6, Some(&current.package), false),
            AcceptedRollbackPolicy::RestoreExactPreState,
            exact,
        ),
        (
            AcceptedLifecycleOperation::UninstallTeardown,
            state(7, Some(&current.package), false),
            state(8, None, false),
            state(7, Some(&current.package), false),
            AcceptedRollbackPolicy::RemoveOnlyNewTarget,
            exact,
        ),
        (
            AcceptedLifecycleOperation::RepeatUse,
            state(8, Some(&current.package), false),
            state(8, Some(&current.package), false),
            state(8, Some(&current.package), false),
            AcceptedRollbackPolicy::ManualReconciliationOnly,
            exact,
        ),
    ];

    for (operation, before, expected_after, rollback, policy, reconciliation) in rows {
        assert!(
            AcceptedLifecyclePlan::new(
                operation,
                before,
                expected_after,
                rollback,
                policy,
                reconciliation,
            )
            .is_err()
        );
    }
}
