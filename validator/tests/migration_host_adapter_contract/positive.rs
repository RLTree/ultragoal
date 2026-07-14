use super::host_fixture::*;
use crate::migration::product::{
    ApplyOutcomeStatus, ConfinedMigrationEffect, MigrationInputSource, apply_product_plan,
    derive_product_plan, issue_apply_authorization, validate_adopted_registry_bytes,
};
use serde_json::Value;

#[test]
fn descriptor_anchored_capture_is_exact_and_zero_write() {
    let fixture = TestHost::new(TestDisposition::Pending);
    let before = fixture.state_bytes();
    let mut adapters = fixture.open().unwrap();
    let input = adapters.source.capture().unwrap();
    let after = fixture.state_bytes();
    assert_eq!(before, after, "capture must not write durable state");
    assert_eq!(input.inventory(), &fixture.inventory);
    assert_eq!(
        input.registry().bytes(),
        std::fs::read(fixture.repository.join(REGISTRY_PATH)).unwrap()
    );
    let boundary = adapters.boundary_authority().unwrap();
    let plan = derive_product_plan(&input, Some(&boundary)).unwrap();
    assert!(plan.effects().is_empty());
    assert_eq!(adapters.store.effect_counts_for_test(), (0, 0));
}

#[test]
fn retirement_apply_and_replay_are_durable_and_preserve_od009_bytes() {
    let fixture = TestHost::new(TestDisposition::Retirement);
    let mut adapters = fixture.open().unwrap();
    let plan = fixture.derive_plan(&mut adapters);
    assert_eq!(plan.effects().len(), 1);
    let mut authority = adapters.apply_authority(&plan).unwrap();
    let token =
        issue_apply_authorization(&plan, &mut adapters.source, &mut authority, &adapters.store)
            .unwrap();
    let outcome = apply_product_plan(
        &plan,
        &token,
        &mut adapters.source,
        &authority,
        &adapters.store,
        &mut adapters.effects,
    )
    .unwrap();
    assert_eq!(outcome.status(), ApplyOutcomeStatus::Applied);
    assert!(outcome.terminal_proof_sha256().is_some());
    assert_eq!(adapters.store.effect_counts_for_test(), (1, 0));
    assert_eq!(fixture.source_bytes(), fixture.original_source_bytes());

    let replay = apply_product_plan(
        &plan,
        &token,
        &mut adapters.source,
        &authority,
        &adapters.store,
        &mut adapters.effects,
    )
    .unwrap();
    assert_eq!(replay.status(), ApplyOutcomeStatus::AlreadyApplied);
    assert_eq!(replay.operation_id(), outcome.operation_id());
    assert_eq!(adapters.store.effect_counts_for_test(), (1, 0));
    assert_eq!(fixture.source_bytes(), fixture.original_source_bytes());
}

#[test]
fn compatibility_apply_uses_a_semantic_permit_and_preserves_source_bytes() {
    let fixture = TestHost::new(TestDisposition::Compatibility);
    let mut adapters = fixture.open().unwrap();
    let plan = fixture.derive_plan(&mut adapters);
    let mut authority = adapters.apply_authority(&plan).unwrap();
    let token =
        issue_apply_authorization(&plan, &mut adapters.source, &mut authority, &adapters.store)
            .unwrap();
    let outcome = apply_product_plan(
        &plan,
        &token,
        &mut adapters.source,
        &authority,
        &adapters.store,
        &mut adapters.effects,
    )
    .unwrap();
    assert_eq!(outcome.status(), ApplyOutcomeStatus::Applied);
    assert_eq!(adapters.store.effect_counts_for_test(), (1, 0));
    assert_eq!(fixture.source_bytes(), fixture.original_source_bytes());
}

#[test]
fn opening_and_reopening_without_an_operation_leave_state_bytes_unchanged() {
    let fixture = TestHost::new(TestDisposition::Pending);
    let provisioned = fixture.state_bytes();
    {
        let adapters = fixture.open().unwrap();
        assert_eq!(adapters.store.effect_counts_for_test(), (0, 0));
    }
    assert_eq!(fixture.state_bytes(), provisioned);
    {
        let adapters = fixture.open().unwrap();
        assert_eq!(adapters.store.effect_counts_for_test(), (0, 0));
    }
    assert_eq!(fixture.state_bytes(), provisioned);
}

#[test]
fn committed_registry_has_no_adopted_effect_and_cannot_imply_a_host_effect() {
    let bytes = include_bytes!("../../../migration/authority-routes.json");
    validate_adopted_registry_bytes(bytes).unwrap();
    let value: Value = serde_json::from_slice(bytes).unwrap();
    let adopted = value["routes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|route| route["transition"].get("adopted_effect").is_some())
        .count();
    assert_eq!(adopted, 0);

    let fixture = TestHost::new(TestDisposition::Pending);
    let mut adapters = fixture.open().unwrap();
    let input = adapters.source.capture().unwrap();
    let authority = adapters.boundary_authority().unwrap();
    let plan = derive_product_plan(&input, Some(&authority)).unwrap();
    assert!(plan.effects().is_empty());
    assert_eq!(adapters.store.effect_counts_for_test(), (0, 0));
}

#[test]
fn semantic_retirement_effect_rolls_back_without_touching_physical_bytes() {
    let fixture = TestHost::new(TestDisposition::Retirement);
    let mut adapters = fixture.open().unwrap();
    let plan = fixture.derive_plan(&mut adapters);
    let mut authority = adapters.apply_authority(&plan).unwrap();
    let token =
        issue_apply_authorization(&plan, &mut adapters.source, &mut authority, &adapters.store)
            .unwrap();
    adapters.store.fail_on_cas_for_test(1);
    assert!(
        apply_product_plan(
            &plan,
            &token,
            &mut adapters.source,
            &authority,
            &adapters.store,
            &mut adapters.effects,
        )
        .is_err()
    );
    let effect = &plan.effects()[0];
    let operation_id = adapters
        .store
        .only_operation_for_test()
        .operation_id()
        .to_owned();
    let applied = adapters.effects.apply(&operation_id, effect, None).unwrap();
    assert_eq!(applied.authority(), effect.after());
    assert_eq!(adapters.store.effect_counts_for_test(), (1, 0));
    let rolled_back = adapters.effects.rollback(&operation_id, effect).unwrap();
    assert_eq!(rolled_back.authority(), effect.before());
    assert_eq!(adapters.store.effect_counts_for_test(), (1, 1));
    assert_eq!(fixture.source_bytes(), fixture.original_source_bytes());
}
