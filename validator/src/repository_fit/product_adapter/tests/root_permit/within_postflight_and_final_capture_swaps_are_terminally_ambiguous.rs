use super::*;

#[test]
pub(crate) fn within_postflight_and_final_capture_swaps_are_terminally_ambiguous() {
    for phase in ["postflight", "final"] {
        let fixture = Fixture::new(&format!("within-{phase}-capture-swap"));
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
                &nonce(&format!("within-{phase}-capture-swap")),
            )
            .unwrap();
        let active = fixture.root.join("agent-standards");
        let displaced = fixture.container.join(format!("within-{phase}-capture-a"));
        let attacker = Arc::new(Mutex::new(None::<JoinHandle<()>>));
        let attacker_slot = Arc::clone(&attacker);
        let arm = move || {
            let handle = arm_capture_barrier(
                TargetCapturePhase::AfterParentHeldBeforeDescend,
                "agent-standards",
                move || replace_directory_preserving_children(&active, &displaced),
            );
            *attacker_slot.lock().unwrap() = Some(handle);
        };
        if phase == "postflight" {
            before_postflight_observation_for_test(arm);
        } else {
            before_final_green_observation_for_test(arm);
        }
        let failure = expect_apply_failure(apply_with_root_permit(
            &context,
            request,
            Some(permit),
            Some(lease),
            10,
        ));
        attacker.lock().unwrap().take().unwrap().join().unwrap();
        assert_target_capture_hook_consumed_for_test();
        assert_eq!(failure.error().id(), AdapterErrorId::ApplyOutcomeAmbiguous);
        assert!(failure.effect_started());
        assert!(!failure.rollback_complete());
    }
}

#[test]
pub(crate) fn linked_special_and_case_aliased_targets_refuse_before_effect() {
    for kind in ["symlink", "hardlink", "fifo", "socket"] {
        let fixture = Fixture::new(&format!("special-{kind}"));
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
                &nonce(kind),
            )
            .unwrap();
        let target = fixture.root.join("AGENTS.md");
        let mut socket = None;
        match kind {
            "symlink" => symlink("elsewhere", &target).unwrap(),
            "hardlink" => {
                fixture.write("hardlink-source", b"linked");
                fs::hard_link(fixture.root.join("hardlink-source"), &target).unwrap();
            }
            "fifo" => {
                let path = std::ffi::CString::new(target.as_os_str().as_encoded_bytes()).unwrap();
                assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
            }
            "socket" => socket = Some(std::os::unix::net::UnixListener::bind(&target).unwrap()),
            _ => unreachable!(),
        }
        let failure = expect_apply_failure(apply_with_root_permit(
            &context,
            request,
            Some(permit),
            Some(lease),
            10,
        ));
        assert!(!failure.effect_started(), "{kind}");
        drop(socket);
    }

    let alias = Fixture::new("case-alias-target");
    alias.write("agents.md", b"alias");
    let context = alias.context();
    let failure = plan_target(&context).unwrap_err();
    assert_eq!(failure.id(), AdapterErrorId::TargetUnavailable);
    assert!(CanonicalPath::parse("../AGENTS.md").is_err());
}

#[test]
pub(crate) fn conflict_and_unowned_overwrite_never_reach_permit_issuance() {
    let fixture = Fixture::new("unowned-conflict");
    fixture.write("AGENTS.md", b"user-owned bytes\n");
    let context = fixture.context();
    let record = plan_target(&context).unwrap();
    assert_eq!(record.conflict_count(), 1);
    let failure = match prepare_apply_request(
        &context,
        &record.to_machine_bytes().unwrap(),
        record.plan_sha256(),
    ) {
        Err(failure) => failure,
        Ok(_) => panic!("conflicting plan unexpectedly prepared an apply request"),
    };
    assert_eq!(failure.id(), AdapterErrorId::PlanConflict);
}

#[test]
pub(crate) fn protected_unowned_state_drift_and_out_of_plan_effects_refuse_without_mutation() {
    let fixture = Fixture::new("protected-state-drift");
    fixture.write("private/unowned.txt", b"original protected bytes\n");
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
            &nonce("protected-state-drift"),
        )
        .unwrap();
    fixture.write("private/unowned.txt", b"substituted protected bytes\n");
    let before = snapshot(&fixture.root);
    let failure = expect_apply_failure(apply_with_root_permit(
        &context,
        request,
        Some(permit),
        Some(lease),
        10,
    ));
    assert!(!failure.effect_started());
    assert_eq!(snapshot(&fixture.root), before);

    let fixture = Fixture::new("out-of-plan-scope");
    let context = fixture.context();
    let request = fixture.request(&context);
    let before = snapshot(&fixture.root);
    let (error, scope_violation) =
        scope_violation_for_test(&fixture.root, &request, fixture.effects(&request));
    assert_eq!(error, FitErrorId::Unauthorized);
    assert!(scope_violation);
    assert_eq!(snapshot(&fixture.root), before);
}
