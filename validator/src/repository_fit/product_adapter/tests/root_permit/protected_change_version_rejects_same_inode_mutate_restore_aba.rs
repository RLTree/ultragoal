use super::*;

#[test]
pub(crate) fn protected_change_version_rejects_same_inode_mutate_restore_aba() {
    let fixture = Fixture::new("protected-mutate-restore-aba");
    fixture.write("private/aba.txt", b"aba-original\n");
    let path = fixture.root.join("private/aba.txt");
    let inode = fs::metadata(&path).unwrap().ino();
    let version = change_version(&path);
    let context = fixture.context();
    let request = fixture.request(&context);
    let attack_path = path.clone();
    let attacker = arm_protected_capture_barrier(
        ProtectedCaptureBoundary::PermitIssuance,
        ProtectedCapturePhase::BeforeFinalRecheck,
        b"",
        move || {
            fs::write(&attack_path, b"aba-mutated!\n").unwrap();
            fs::write(attack_path, b"aba-original\n").unwrap();
        },
    );
    let result = new_authority().issue(
        &context,
        &request,
        fixture.effects(&request),
        10,
        20,
        &nonce("protected-mutate-restore-aba"),
    );
    assert_protected_capture_hook_consumed_for_test();
    attacker.join().unwrap();
    let failure = match result {
        Err(failure) => failure,
        Ok(_) => panic!("a protected mutate-restore ABA issued authority"),
    };
    assert_eq!(failure.id(), AdapterErrorId::TargetUnavailable);
    assert_late_regular_write_remains(&path, inode, b"aba-original\n", version);
}

#[test]
pub(crate) fn protected_descriptor_capture_rejects_permit_issuance_ab_swap() {
    let fixture = Fixture::new("protected-permit-issuance-ab-swap");
    let (active, replacement, displaced) = protected_directory_swap(&fixture, "permit-issuance");
    let context = fixture.context();
    let request = fixture.request(&context);
    let attacker = arm_protected_capture_barrier(
        ProtectedCaptureBoundary::PermitIssuance,
        ProtectedCapturePhase::AfterEnumerationBeforeChildOpen,
        b"private",
        move || swap_protected_directories(active, replacement, displaced),
    );
    let result = new_authority().issue(
        &context,
        &request,
        fixture.effects(&request),
        10,
        20,
        &nonce("protected-permit-issuance-ab-swap"),
    );
    assert_protected_capture_hook_consumed_for_test();
    attacker.join().unwrap();
    let failure = match result {
        Err(failure) => failure,
        Ok(_) => panic!("a protected A/B issuance hybrid unexpectedly issued authority"),
    };
    assert_eq!(failure.id(), AdapterErrorId::TargetUnavailable);
    assert!(
        CANONICAL_TEMPLATES
            .iter()
            .all(|row| !fixture.root.join(row.target_path).is_file())
    );
}

#[test]
pub(crate) fn protected_descriptor_capture_rejects_immediate_pre_effect_ab_swap() {
    let fixture = Fixture::new("protected-pre-effect-ab-swap");
    let (active, replacement, displaced) =
        protected_directory_swap(&fixture, "immediate-pre-effect");
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
            &nonce("protected-pre-effect-ab-swap"),
        )
        .unwrap();
    let attacker = arm_protected_capture_barrier(
        ProtectedCaptureBoundary::ImmediatePreEffect,
        ProtectedCapturePhase::AfterDirectoryHeldBeforeDescend,
        b"private",
        move || swap_protected_directories(active, replacement, displaced),
    );
    let result = apply_with_root_permit(&context, request, Some(permit), Some(lease), 10);
    assert_protected_capture_hook_consumed_for_test();
    attacker.join().unwrap();
    let failure = expect_apply_failure(result);
    assert_eq!(failure.error().id(), AdapterErrorId::TargetUnavailable);
    assert!(!failure.effect_started());
}

#[test]
pub(crate) fn protected_descriptor_capture_cannot_hide_postflight_undeclared_write() {
    let fixture = Fixture::new("protected-postflight-hidden-write");
    let (active, replacement, displaced) =
        protected_directory_swap(&fixture, "postflight-hidden-write");
    let displaced_probe = displaced.clone();
    let context = fixture.context();
    let request = fixture.request(&context);
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            UndeclaredWrite {
                inner: fixture.effects(&request),
                root: fixture.root.clone(),
                fired: false,
            },
            10,
            20,
            &nonce("protected-postflight-hidden-write"),
        )
        .unwrap();
    let attacker = arm_protected_capture_barrier(
        ProtectedCaptureBoundary::Postflight,
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
    assert!(displaced_probe.join("undeclared-private-canary").is_file());
    assert!(
        !failure
            .error()
            .to_string()
            .contains("do-not-echo-this-private-canary")
    );
}

#[test]
pub(crate) fn protected_descriptor_capture_rejects_final_green_ab_swap() {
    let fixture = Fixture::new("protected-final-green-ab-swap");
    let (active, replacement, displaced) = protected_directory_swap(&fixture, "final-green");
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
            &nonce("protected-final-green-ab-swap"),
        )
        .unwrap();
    let attacker = arm_protected_capture_barrier(
        ProtectedCaptureBoundary::FinalGreen,
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
