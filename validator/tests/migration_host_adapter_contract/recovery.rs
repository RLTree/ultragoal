use super::support::*;
use crate::migration::product::{
    ApplyOutcomeStatus, apply_product_plan, issue_apply_authorization, recover_product_operation,
};

fn crash_reopen_and_recover(fail_on_cas: usize, applies_before_reopen: u64) {
    let fixture = TestHost::new(TestDisposition::Retirement);
    let mut adapters = fixture.open().unwrap();
    let plan = fixture.derive_plan(&mut adapters);
    let mut authority = adapters.apply_authority(&plan).unwrap();
    let token =
        issue_apply_authorization(&plan, &mut adapters.source, &mut authority, &adapters.store)
            .unwrap();
    adapters.store.fail_on_cas_for_test(fail_on_cas);
    let error = apply_product_plan(
        &plan,
        &token,
        &mut adapters.source,
        &authority,
        &adapters.store,
        &mut adapters.effects,
    )
    .unwrap_err();
    assert_eq!(error.code(), "migration-product-operation-interrupted");
    assert_eq!(
        adapters.store.effect_counts_for_test().0,
        applies_before_reopen
    );
    let operation = adapters.store.only_operation_for_test();
    let operation_id = operation.operation_id().to_owned();
    drop(operation);
    drop(authority);
    drop(adapters);

    let mut reopened = fixture.open().unwrap();
    let recovery_authority = reopened.recovery_authority(&operation_id, &plan).unwrap();
    let outcome = recover_product_operation(
        &operation_id,
        &plan,
        &mut reopened.source,
        &recovery_authority,
        &reopened.store,
        &mut reopened.effects,
    )
    .unwrap();
    assert_eq!(outcome.status(), ApplyOutcomeStatus::AlreadyApplied);
    assert!(outcome.terminal_proof_sha256().is_some());
    assert_eq!(reopened.store.effect_counts_for_test(), (1, 0));
    assert_eq!(fixture.source_bytes(), fixture.original_source_bytes());

    let replay = recover_product_operation(
        &operation_id,
        &plan,
        &mut reopened.source,
        &recovery_authority,
        &reopened.store,
        &mut reopened.effects,
    )
    .unwrap();
    assert_eq!(replay.status(), ApplyOutcomeStatus::AlreadyApplied);
    assert_eq!(reopened.store.effect_counts_for_test(), (1, 0));
}

#[test]
fn crash_before_effect_recovers_after_reopen_without_duplicate() {
    crash_reopen_and_recover(1, 0);
}

#[test]
fn crash_after_effect_recovers_after_reopen_without_duplicate() {
    crash_reopen_and_recover(2, 1);
}

#[test]
fn process_lock_is_nonblocking_and_reopen_succeeds_after_release() {
    let fixture = TestHost::new(TestDisposition::Retirement);
    let first = fixture.open().unwrap();
    let error = match fixture.open() {
        Ok(_) => panic!("second host unexpectedly acquired the process lock"),
        Err(error) => error,
    };
    assert_eq!(error.code(), "migration-host-lock-busy");
    drop(first);
    let reopened = fixture.open().unwrap();
    assert_eq!(reopened.store.effect_counts_for_test(), (0, 0));
}

#[test]
fn durable_ledger_is_canonical_and_reopens_after_terminal_apply() {
    let fixture = TestHost::new(TestDisposition::Retirement);
    let mut adapters = fixture.open().unwrap();
    let plan = fixture.derive_plan(&mut adapters);
    let mut authority = adapters.apply_authority(&plan).unwrap();
    let token =
        issue_apply_authorization(&plan, &mut adapters.source, &mut authority, &adapters.store)
            .unwrap();
    apply_product_plan(
        &plan,
        &token,
        &mut adapters.source,
        &authority,
        &adapters.store,
        &mut adapters.effects,
    )
    .unwrap();
    let bytes = fixture.state_bytes();
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["schema_version"], "DarwinMigrationHostLedger-v1");
    assert!(
        value["ledger_sha256"]
            .as_str()
            .is_some_and(|digest| digest.starts_with("sha256:"))
    );
    drop(authority);
    drop(adapters);
    let reopened = fixture.open().unwrap();
    assert_eq!(reopened.store.effect_counts_for_test(), (1, 0));
    assert_eq!(fixture.state_bytes(), bytes);
}

#[test]
fn compatibility_effect_reconciles_after_reopen_and_deadline_crossing() {
    let fixture = TestHost::new(TestDisposition::Compatibility);
    let mut adapters = fixture.open().unwrap();
    let plan = fixture.derive_plan(&mut adapters);
    let mut authority = adapters.apply_authority(&plan).unwrap();
    let token =
        issue_apply_authorization(&plan, &mut adapters.source, &mut authority, &adapters.store)
            .unwrap();
    adapters.store.fail_on_cas_for_test(3);
    let error = apply_product_plan(
        &plan,
        &token,
        &mut adapters.source,
        &authority,
        &adapters.store,
        &mut adapters.effects,
    )
    .unwrap_err();
    assert_eq!(error.code(), "migration-product-operation-interrupted");
    assert_eq!(adapters.store.effect_counts_for_test(), (1, 0));
    let operation_id = adapters
        .store
        .only_operation_for_test()
        .operation_id()
        .to_owned();
    drop(authority);
    drop(adapters);

    let mut reopened = fixture.open().unwrap();
    reopened.advance_trusted_time_for_test(2 * 86_400_000);
    let recovery_authority = reopened.recovery_authority(&operation_id, &plan).unwrap();
    let outcome = recover_product_operation(
        &operation_id,
        &plan,
        &mut reopened.source,
        &recovery_authority,
        &reopened.store,
        &mut reopened.effects,
    )
    .unwrap();
    assert_eq!(outcome.status(), ApplyOutcomeStatus::AlreadyApplied);
    assert_eq!(reopened.store.effect_counts_for_test(), (1, 0));
    assert_eq!(fixture.source_bytes(), fixture.original_source_bytes());
}
