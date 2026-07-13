use super::support::*;
use crate::migration::product::{
    apply_product_plan, derive_product_plan, issue_apply_authorization, recover_product_operation,
    ApplyOutcomeStatus, JournalPhase,
};
use crate::migration::SurfaceStatus;

fn run_crash_and_recover(fail_on_cas: usize, expected_applies_before_recovery: usize) {
    let input = retirement_input();
    let plan = derive_plan(&input).unwrap();
    let store = FakeStore::default();
    store.fail_on_cas(fail_on_cas);
    let mut source = FakeSource::new(input);
    let mut authority = FakeAuthority::current(&plan, 'a');
    let token = issue_apply_authorization(&plan, &mut source, &mut authority, &store).unwrap();
    let mut effects = FakeEffects::for_plan(&plan);
    assert_eq!(
        apply_product_plan(&plan, &token, &mut source, &authority, &store, &mut effects,)
            .unwrap_err()
            .code(),
        "migration-product-operation-interrupted"
    );
    assert_eq!(effects.counts().0, expected_applies_before_recovery);
    let interrupted = store.only_operation();
    let outcome = recover_product_operation(
        interrupted.operation_id(),
        &plan,
        &mut source,
        &authority,
        &store,
        &mut effects,
    )
    .unwrap();
    assert_eq!(outcome.status(), ApplyOutcomeStatus::AlreadyApplied);
    assert!(outcome.terminal_proof_sha256().is_some());
    assert_eq!(
        effects.counts().0,
        1,
        "recovery must never duplicate effect"
    );
}

#[test]
fn recovery_handles_crash_before_effect() {
    run_crash_and_recover(1, 0);
}

#[test]
fn recovery_handles_crash_after_effect_before_effect_checkpoint() {
    run_crash_and_recover(2, 1);
}

#[test]
fn recovery_handles_crash_before_terminal_checkpoint() {
    run_crash_and_recover(4, 1);
}

#[test]
fn reserved_operation_recovers_after_apply_authorization_expires() {
    let input = retirement_input();
    let plan = derive_plan(&input).unwrap();
    let store = FakeStore::default();
    store.fail_on_cas(2);
    let mut source = FakeSource::new(input);
    let mut authority = FakeAuthority::current(&plan, 'b');
    let token = issue_apply_authorization(&plan, &mut source, &mut authority, &store).unwrap();
    let mut effects = FakeEffects::for_plan(&plan);
    assert!(
        apply_product_plan(&plan, &token, &mut source, &authority, &store, &mut effects,).is_err()
    );
    authority.now = authority.expires + 100;
    let interrupted = store.only_operation();
    let outcome = recover_product_operation(
        interrupted.operation_id(),
        &plan,
        &mut source,
        &authority,
        &store,
        &mut effects,
    )
    .unwrap();
    assert_eq!(outcome.status(), ApplyOutcomeStatus::AlreadyApplied);
}

fn two_effect_input() -> crate::migration::product::ProductInputSnapshot {
    input_with_routes(
        vec![
            surface(
                "LEGACY-SKILL:a",
                "legacy-skill",
                "skills/a/SKILL.md",
                'a',
                SurfaceStatus::Candidate,
                &["reader-a"],
                &[],
                &[],
                &[],
            ),
            surface(
                "LEGACY-SKILL:b",
                "legacy-skill",
                "skills/b/SKILL.md",
                'c',
                SurfaceStatus::Candidate,
                &["reader-b"],
                &[],
                &[],
                &[],
            ),
            surface(
                "SKILL:current",
                "skill",
                "skills/current/SKILL.md",
                'b',
                SurfaceStatus::Active,
                &[],
                &[],
                &["current-public"],
                &[],
            ),
        ],
        vec![
            route(
                "route-a",
                "LEGACY-SKILL:a",
                "skills/a/SKILL.md",
                "SKILL:current",
                'a',
                'b',
                Some("retirement"),
            ),
            route(
                "route-b",
                "LEGACY-SKILL:b",
                "skills/b/SKILL.md",
                "SKILL:current",
                'c',
                'b',
                Some("retirement"),
            ),
        ],
        '0',
    )
}

#[test]
fn rejected_later_effect_rolls_back_prior_effect_in_reverse() {
    let input = two_effect_input();
    let plan = derive_plan(&input).unwrap();
    let store = FakeStore::default();
    let mut source = FakeSource::new(input);
    let mut authority = FakeAuthority::current(&plan, 'c');
    let token = issue_apply_authorization(&plan, &mut source, &mut authority, &store).unwrap();
    let mut effects = FakeEffects::for_plan(&plan);
    effects.reject(plan.effects()[1].effect_id());
    let outcome =
        apply_product_plan(&plan, &token, &mut source, &authority, &store, &mut effects).unwrap();
    assert_eq!(outcome.status(), ApplyOutcomeStatus::RolledBack);
    assert_eq!(effects.counts(), (1, 1));
    let operation = store.only_operation();
    assert_eq!(operation.phase(), JournalPhase::TerminalRolledBack);
}

#[test]
fn rejected_response_after_a_side_effect_rolls_back_current_and_prior_effects() {
    let input = two_effect_input();
    let plan = derive_plan(&input).unwrap();
    let store = FakeStore::default();
    let mut source = FakeSource::new(input);
    let mut authority = FakeAuthority::current(&plan, 'e');
    let token = issue_apply_authorization(&plan, &mut source, &mut authority, &store).unwrap();
    let mut effects = FakeEffects::for_plan(&plan);
    effects.reject_after_effect(plan.effects()[1].effect_id());
    let outcome =
        apply_product_plan(&plan, &token, &mut source, &authority, &store, &mut effects).unwrap();
    assert_eq!(outcome.status(), ApplyOutcomeStatus::RolledBack);
    assert_eq!(effects.counts(), (2, 2));
    let operation = store.only_operation();
    assert_eq!(operation.phase(), JournalPhase::TerminalRolledBack);
    assert!(outcome.terminal_proof_sha256().is_some());
}

#[test]
fn ambiguous_effect_never_fabricates_rollback_or_terminal_retirement() {
    let input = two_effect_input();
    let plan = derive_plan(&input).unwrap();
    let store = FakeStore::default();
    let mut source = FakeSource::new(input);
    let mut authority = FakeAuthority::current(&plan, 'd');
    let token = issue_apply_authorization(&plan, &mut source, &mut authority, &store).unwrap();
    let mut effects = FakeEffects::for_plan(&plan);
    effects.ambiguous(plan.effects()[1].effect_id());
    let outcome =
        apply_product_plan(&plan, &token, &mut source, &authority, &store, &mut effects).unwrap();
    assert_eq!(outcome.status(), ApplyOutcomeStatus::Ambiguous);
    assert!(outcome.terminal_proof_sha256().is_none());
    assert_eq!(effects.counts(), (1, 0));
}

#[test]
fn compatibility_recovery_before_effect_refuses_after_deadline_crossing_with_zero_effect() {
    let input = compatibility_input();
    let plan = derive_plan(&input).unwrap();
    let store = FakeStore::default();
    store.fail_on_cas(1);
    let mut source = FakeSource::new(input);
    let mut authority = FakeAuthority::current(&plan, 'a');
    let token = issue_apply_authorization(&plan, &mut source, &mut authority, &store).unwrap();
    let mut effects = FakeEffects::for_plan(&plan);
    assert_eq!(
        apply_product_plan(&plan, &token, &mut source, &authority, &store, &mut effects,)
            .unwrap_err()
            .code(),
        "migration-product-operation-interrupted"
    );
    assert_eq!(effects.counts(), (0, 0));
    authority.boundary_sequence += 100;
    authority.boundary_now = 86_402_000;
    let operation = store.only_operation();
    assert_eq!(
        recover_product_operation(
            operation.operation_id(),
            &plan,
            &mut source,
            &authority,
            &store,
            &mut effects,
        )
        .unwrap_err()
        .code(),
        "migration-product-compatibility-boundary-crossed"
    );
    assert_eq!(effects.counts(), (0, 0));
    assert_eq!(
        effects.authority(plan.effects()[0].effect_id()),
        plan.effects()[0].before().clone()
    );
}

#[test]
fn compatibility_recovery_reconciles_pre_boundary_effect_without_duplicate_after_crossing() {
    let input = compatibility_input();
    let plan = derive_plan(&input).unwrap();
    let store = FakeStore::default();
    store.fail_on_cas(3);
    let mut source = FakeSource::new(input);
    let mut authority = FakeAuthority::current(&plan, 'b');
    let token = issue_apply_authorization(&plan, &mut source, &mut authority, &store).unwrap();
    let mut effects = FakeEffects::for_plan(&plan);
    assert_eq!(
        apply_product_plan(&plan, &token, &mut source, &authority, &store, &mut effects,)
            .unwrap_err()
            .code(),
        "migration-product-operation-interrupted"
    );
    assert_eq!(effects.counts(), (1, 0));
    authority.boundary_sequence += 100;
    authority.boundary_now = 86_402_000;
    let operation = store.only_operation();
    let outcome = recover_product_operation(
        operation.operation_id(),
        &plan,
        &mut source,
        &authority,
        &store,
        &mut effects,
    )
    .unwrap();
    assert_eq!(outcome.status(), ApplyOutcomeStatus::AlreadyApplied);
    assert_eq!(effects.counts(), (1, 0));

    let replay = recover_product_operation(
        operation.operation_id(),
        &plan,
        &mut source,
        &authority,
        &store,
        &mut effects,
    )
    .unwrap();
    assert_eq!(replay.status(), ApplyOutcomeStatus::AlreadyApplied);
    assert_eq!(effects.counts(), (1, 0));
}

#[test]
fn compatibility_recovery_with_unbound_completion_timing_is_ambiguous_and_never_duplicates() {
    let input = compatibility_input();
    let plan = derive_plan(&input).unwrap();
    let store = FakeStore::default();
    store.fail_on_cas(3);
    let mut source = FakeSource::new(input);
    let mut authority = FakeAuthority::current(&plan, 'c');
    let token = issue_apply_authorization(&plan, &mut source, &mut authority, &store).unwrap();
    let mut effects = FakeEffects::for_plan(&plan);
    assert!(
        apply_product_plan(&plan, &token, &mut source, &authority, &store, &mut effects,).is_err()
    );
    assert_eq!(effects.counts(), (1, 0));
    effects.substitute_effect_permit(plan.effects()[0].effect_id(), Some(sha('f')));
    authority.boundary_sequence += 100;
    authority.boundary_now = 86_402_000;
    let operation = store.only_operation();
    let outcome = recover_product_operation(
        operation.operation_id(),
        &plan,
        &mut source,
        &authority,
        &store,
        &mut effects,
    )
    .unwrap();
    assert_eq!(outcome.status(), ApplyOutcomeStatus::Ambiguous);
    assert_eq!(effects.counts(), (1, 0));
    assert!(outcome.terminal_proof_sha256().is_none());
}
