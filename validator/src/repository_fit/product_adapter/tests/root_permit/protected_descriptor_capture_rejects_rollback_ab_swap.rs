use super::*;

#[test]
pub(crate) fn protected_descriptor_capture_rejects_rollback_ab_swap() {
    let fixture = Fixture::new("protected-rollback-ab-swap");
    let (active, replacement, displaced) = protected_directory_swap(&fixture, "rollback");
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
            &nonce("protected-rollback-ab-swap"),
        )
        .unwrap();
    let attacker = arm_protected_capture_barrier(
        ProtectedCaptureBoundary::Rollback,
        ProtectedCapturePhase::AfterEnumerationBeforeChildOpen,
        b"private",
        move || swap_protected_directories(active, replacement, displaced),
    );
    let result = apply_with_root_permit(&context, request, Some(permit), Some(lease), 10);
    assert_protected_capture_hook_consumed_for_test();
    attacker.join().unwrap();
    let failure = expect_apply_failure(result);
    assert_eq!(failure.error().id(), AdapterErrorId::ApplyOutcomeAmbiguous);
    assert!(failure.effect_started());
    assert!(!failure.rollback_complete());
}

#[test]
pub(crate) fn protected_descriptor_capture_rejects_reconciliation_ab_swap() {
    let fixture = Fixture::new("protected-reconciliation-ab-swap");
    let (active, replacement, displaced) =
        protected_directory_swap(&fixture, "ambiguity-reconciliation");
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
            &nonce("protected-reconciliation-ab-swap"),
        )
        .unwrap();
    let attacker = arm_protected_capture_barrier(
        ProtectedCaptureBoundary::AmbiguityReconciliation,
        ProtectedCapturePhase::AfterDirectoryHeldBeforeDescend,
        b"private",
        move || swap_protected_directories(active, replacement, displaced),
    );
    let result = apply_with_root_permit(&context, request, Some(permit), Some(lease), 10);
    assert_protected_capture_hook_consumed_for_test();
    attacker.join().unwrap();
    let failure = expect_apply_failure(result);
    assert_eq!(failure.error().id(), AdapterErrorId::ApplyOutcomeAmbiguous);
    assert!(failure.effect_started());
    assert!(!failure.rollback_complete());
}

#[test]
pub(crate) fn protected_pre_effect_late_length_change_refuses_without_effect() {
    let fixture = Fixture::new("protected-pre-effect-late-length-write");
    fixture.write("private/length.txt", b"short\n");
    let path = fixture.root.join("private/length.txt");
    let inode = fs::metadata(&path).unwrap().ino();
    let version = change_version(&path);
    let context = fixture.context();
    let request = fixture.request(&context);
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            fixture.effects(&request),
            10,
            20,
            &nonce("protected-pre-effect-late-length-write"),
        )
        .unwrap();
    let attack_path = path.clone();
    let attacker = arm_protected_capture_barrier(
        ProtectedCaptureBoundary::ImmediatePreEffect,
        ProtectedCapturePhase::AfterRegularRowRevalidated,
        b"private/length.txt",
        move || fs::write(attack_path, b"late length expansion remains\n").unwrap(),
    );
    let result = apply_with_root_permit(&context, request, Some(permit), Some(lease), 10);
    assert_protected_capture_hook_consumed_for_test();
    attacker.join().unwrap();
    let failure = expect_apply_failure(result);
    assert_eq!(failure.error().id(), AdapterErrorId::TargetUnavailable);
    assert!(!failure.effect_started());
    assert_late_regular_write_remains(&path, inode, b"late length expansion remains\n", version);
}

#[test]
pub(crate) fn protected_postflight_late_mode_change_cannot_produce_success_or_rollback() {
    let fixture = Fixture::new("protected-postflight-late-mode-change");
    fixture.write("private/mode.txt", b"mode protected\n");
    let path = fixture.root.join("private/mode.txt");
    let inode = fs::metadata(&path).unwrap().ino();
    let version = change_version(&path);
    let context = fixture.context();
    let request = fixture.request(&context);
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            fixture.effects(&request),
            10,
            20,
            &nonce("protected-postflight-late-mode-change"),
        )
        .unwrap();
    let attack_path = path.clone();
    let attacker = arm_protected_capture_barrier(
        ProtectedCaptureBoundary::Postflight,
        ProtectedCapturePhase::AfterRegularRowRevalidated,
        b"private/mode.txt",
        move || {
            fs::set_permissions(attack_path, fs::Permissions::from_mode(0o600)).unwrap();
        },
    );
    let result = apply_with_root_permit(&context, request, Some(permit), Some(lease), 10);
    assert_protected_capture_hook_consumed_for_test();
    attacker.join().unwrap();
    let failure = expect_apply_failure(result);
    assert_eq!(failure.error().id(), AdapterErrorId::ApplyOutcomeAmbiguous);
    assert!(failure.effect_started());
    assert!(!failure.rollback_complete());
    assert_late_regular_write_remains(&path, inode, b"mode protected\n", version);
    assert_eq!(fs::metadata(path).unwrap().mode() & 0o7777, 0o600);
}

#[test]
pub(crate) fn protected_final_green_target_overlap_is_caught_by_the_after_collect() {
    let fixture = Fixture::new("protected-final-green-recheck-late-write");
    fixture.write("private/final.txt", b"final-before\n");
    let path = fixture.root.join("private/final.txt");
    let inode = fs::metadata(&path).unwrap().ino();
    let version = change_version(&path);
    let context = fixture.context();
    let request = fixture.request(&context);
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            fixture.effects(&request),
            10,
            20,
            &nonce("protected-final-green-recheck-late-write"),
        )
        .unwrap();
    let attack_path = path.clone();
    let attacker = arm_protected_capture_barrier(
        ProtectedCaptureBoundary::FinalGreenRecheck,
        ProtectedCapturePhase::AfterRegularRowRevalidated,
        b"private/final.txt",
        move || fs::write(attack_path, b"final-after!\n").unwrap(),
    );
    let result = apply_with_root_permit(&context, request, Some(permit), Some(lease), 10);
    assert_protected_capture_hook_consumed_for_test();
    attacker.join().unwrap();
    let failure = expect_apply_failure(result);
    assert_eq!(failure.error().id(), AdapterErrorId::ApplyOutcomeAmbiguous);
    assert!(failure.effect_started());
    assert!(!failure.rollback_complete());
    assert_late_regular_write_remains(&path, inode, b"final-after!\n", version);
}
