use crate::lifecycle_fixture::{Fixture, installed, lifecycle, request};
use ultragoal::plugin_product::lifecycle::{
    ApplyDisposition, LifecycleIntent, LifecycleState, verify,
};

#[test]
fn real_adapter_completes_all_eight_lifecycle_intents_in_one_isolated_root() {
    let fixture = Fixture::new("all-positive");
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
    let mut operation = fixture.operation(&v11, &fresh);
    let report = operation.apply(&empty, &fresh).unwrap();
    assert_eq!(report.disposition, ApplyDisposition::Applied);
    verify(&report.state, &fresh).unwrap();
    assert_eq!(operation.observe_state().unwrap(), report.state);
    let mut state = report.state;

    let update = lifecycle(
        &state,
        request(
            LifecycleIntent::MonotonicUpdate,
            Some(v12.authority.clone()),
            None,
            state.installed.as_ref(),
            true,
            false,
        ),
    );
    let mut operation = fixture.operation(&v12, &update);
    state = operation.apply(&state, &update).unwrap().state;
    assert_eq!(state, installed(&v12.authority, 2));

    let rollback = lifecycle(
        &state,
        request(
            LifecycleIntent::AuthorizedRollback,
            Some(v11.authority.clone()),
            None,
            state.installed.as_ref(),
            true,
            true,
        ),
    );
    let mut operation = fixture.operation(&v11, &rollback);
    state = operation.apply(&state, &rollback).unwrap().state;
    assert_eq!(state, installed(&v11.authority, 3));

    for intent in [
        LifecycleIntent::IdempotentReinstall,
        LifecycleIntent::RepeatUse,
    ] {
        let read_only = lifecycle(
            &state,
            request(
                intent,
                Some(v11.authority.clone()),
                None,
                state.installed.as_ref(),
                false,
                false,
            ),
        );
        let before = fixture.tree();
        let mut operation = fixture.operation(&v11, &read_only);
        let report = operation.apply(&state, &read_only).unwrap();
        assert_eq!(report.state, state);
        assert_eq!(operation.observed_mutation_count(), 0);
        assert_eq!(fixture.tree(), before, "read-only lifecycle mutated tree");
    }

    fixture.replace(
        "cache/harness-ultragoal.hugpkg",
        Some(&v11.authority.package_sha256),
        Some(v12.snapshot.archive()),
    );
    let stale = LifecycleState {
        cache: Some(v12.authority.clone()),
        ..state.clone()
    };
    let refresh = lifecycle(
        &stale,
        request(
            LifecycleIntent::StaleCacheRecovery,
            None,
            None,
            stale.installed.as_ref(),
            true,
            false,
        ),
    );
    let mut operation = fixture.operation(&v11, &refresh);
    state = operation.apply(&stale, &refresh).unwrap().state;
    assert_eq!(state.installed, state.cache);

    fixture.replace(
        "installed/harness-ultragoal.hugpkg",
        Some(&v11.authority.package_sha256),
        Some(v12.snapshot.archive()),
    );
    let interrupted = LifecycleState {
        installed: Some(v12.authority.clone()),
        cache: Some(v11.authority.clone()),
        generation: state.generation + 1,
        recovery_required: true,
    };
    let recover_prior = lifecycle(
        &interrupted,
        request(
            LifecycleIntent::FailedUpdateRecovery,
            None,
            Some(state.clone()),
            interrupted.installed.as_ref(),
            true,
            true,
        ),
    );
    let mut operation = fixture.operation(&v11, &recover_prior);
    state = operation.apply(&interrupted, &recover_prior).unwrap().state;
    assert_eq!(state.installed, Some(v11.authority.clone()));

    let uninstall = lifecycle(
        &state,
        request(
            LifecycleIntent::UninstallTeardown,
            None,
            None,
            state.installed.as_ref(),
            true,
            false,
        ),
    );
    let mut operation = fixture.operation(&v11, &uninstall);
    let removed = operation.apply(&state, &uninstall).unwrap().state;
    assert!(removed.installed.is_none() && removed.cache.is_none());
    assert_eq!(operation.observe_state().unwrap(), removed);
}
