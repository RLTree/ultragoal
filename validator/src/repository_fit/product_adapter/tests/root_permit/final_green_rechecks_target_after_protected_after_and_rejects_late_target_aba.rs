use super::*;

#[test]
pub(crate) fn final_green_rechecks_target_after_protected_after_and_rejects_late_target_aba() {
    let fixture = Fixture::new("target-final-green-post-protected-aba");
    fixture.install_all();
    let row = CANONICAL_TEMPLATES
        .iter()
        .find(|row| row.target_path == "AGENTS.md")
        .unwrap();
    let path = fixture.root.join(row.target_path);
    let inode = fs::metadata(&path).unwrap().ino();
    let version = change_version(&path);
    let context = fixture.context();
    let request = fixture.request(&context);
    assert!(request.plan.mutations.is_empty());
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            fixture.effects(&request),
            10,
            20,
            &nonce("target-final-green-post-protected-aba"),
        )
        .unwrap();
    let attack_path = path.clone();
    let other = same_length_other(row.bytes);
    let attacker = arm_protected_capture_barrier(
        ProtectedCaptureBoundary::FinalGreenRecheck,
        ProtectedCapturePhase::BeforeFinalRecheck,
        b"",
        move || {
            fs::write(&attack_path, other).unwrap();
            fs::write(attack_path, row.bytes).unwrap();
        },
    );
    let result = apply_with_root_permit(&context, request, Some(permit), Some(lease), 10);
    assert_protected_capture_hook_consumed_for_test();
    attacker.join().unwrap();
    let failure = expect_apply_failure(result);
    assert_eq!(failure.error().id(), AdapterErrorId::ApplyOutcomeAmbiguous);
    assert!(failure.effect_started());
    assert!(!failure.rollback_complete());
    assert_late_regular_write_remains(&path, inode, row.bytes, version);
}

#[test]
pub(crate) fn protected_rollback_late_write_withholds_complete_rollback() {
    let fixture = Fixture::new("protected-rollback-late-write");
    fixture.write("private/rollback.txt", b"rollback-before\n");
    let path = fixture.root.join("private/rollback.txt");
    let inode = fs::metadata(&path).unwrap().ino();
    let version = change_version(&path);
    let context = fixture.context();
    let request = fixture.request(&context);
    let mut effects = fixture.effects(&request);
    effects.fail_on_calls([2]);
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            effects,
            10,
            20,
            &nonce("protected-rollback-late-write"),
        )
        .unwrap();
    let attack_path = path.clone();
    let attacker = arm_protected_capture_barrier(
        ProtectedCaptureBoundary::Rollback,
        ProtectedCapturePhase::AfterRegularRowRevalidated,
        b"private/rollback.txt",
        move || fs::write(attack_path, b"rollback-after-is-left\n").unwrap(),
    );
    let result = apply_with_root_permit(&context, request, Some(permit), Some(lease), 10);
    assert_protected_capture_hook_consumed_for_test();
    attacker.join().unwrap();
    let failure = expect_apply_failure(result);
    assert_eq!(failure.error().id(), AdapterErrorId::ApplyOutcomeAmbiguous);
    assert!(failure.effect_started());
    assert!(!failure.rollback_complete());
    assert_late_regular_write_remains(&path, inode, b"rollback-after-is-left\n", version);
}

#[test]
pub(crate) fn protected_reconciliation_late_write_cannot_fabricate_prior_state() {
    let fixture = Fixture::new("protected-reconciliation-late-write");
    fixture.write("private/reconcile.txt", b"reconcile-before\n");
    let path = fixture.root.join("private/reconcile.txt");
    let inode = fs::metadata(&path).unwrap().ino();
    let version = change_version(&path);
    let context = fixture.context();
    let request = fixture.request(&context);
    let mut effects = fixture.effects(&request);
    effects.fail_on_calls([2]);
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            effects,
            10,
            20,
            &nonce("protected-reconciliation-late-write"),
        )
        .unwrap();
    let attack_path = path.clone();
    let attacker = arm_protected_capture_barrier(
        ProtectedCaptureBoundary::AmbiguityReconciliation,
        ProtectedCapturePhase::AfterRegularRowRevalidated,
        b"private/reconcile.txt",
        move || fs::write(attack_path, b"reconcile-after!\n").unwrap(),
    );
    let result = apply_with_root_permit(&context, request, Some(permit), Some(lease), 10);
    assert_protected_capture_hook_consumed_for_test();
    attacker.join().unwrap();
    let failure = expect_apply_failure(result);
    assert_eq!(failure.error().id(), AdapterErrorId::ApplyOutcomeAmbiguous);
    assert!(failure.effect_started());
    assert!(!failure.rollback_complete());
    assert_late_regular_write_remains(&path, inode, b"reconcile-after!\n", version);
}
