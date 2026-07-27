use super::*;

#[test]
pub(crate) fn protected_descendants_inside_managed_ancestors_are_preserved_and_bound() {
    let fixture = Fixture::new("managed-ancestor-protected-descendant");
    fixture.write(
        "agent-standards/unrelated.txt",
        b"preserve this protected descendant\n",
    );
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
            &nonce("managed-ancestor-protected-descendant"),
        )
        .unwrap();
    let outcome = expect_apply_ok(apply_with_root_permit(
        &context,
        request,
        Some(permit),
        Some(lease),
        10,
    ));
    assert_eq!(outcome.status(), "applied");
    assert_eq!(
        fs::read(fixture.root.join("agent-standards/unrelated.txt")).unwrap(),
        b"preserve this protected descendant\n"
    );

    let drift = Fixture::new("managed-ancestor-protected-drift");
    drift.write("agent-standards/unrelated.txt", b"protected before\n");
    let context = drift.context();
    let request = drift.request(&context);
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            drift.effects(&request),
            10,
            20,
            &nonce("managed-ancestor-protected-drift"),
        )
        .unwrap();
    drift.write("agent-standards/unrelated.txt", b"protected after\n");
    let failure = expect_apply_failure(apply_with_root_permit(
        &context,
        request,
        Some(permit),
        Some(lease),
        10,
    ));
    assert!(!failure.effect_started());
}

#[test]
pub(crate) fn protected_links_and_special_objects_fail_closed_before_effects() {
    for kind in ["symlink", "hardlink", "fifo", "socket"] {
        let fixture = Fixture::new(&format!("protected-{kind}"));
        fixture.write("private/source", b"protected source\n");
        let candidate = fixture.root.join("private/candidate");
        let mut socket = None;
        match kind {
            "symlink" => symlink(fixture.root.join("private/source"), &candidate).unwrap(),
            "hardlink" => fs::hard_link(fixture.root.join("private/source"), &candidate).unwrap(),
            "fifo" => {
                let path =
                    std::ffi::CString::new(candidate.as_os_str().as_encoded_bytes()).unwrap();
                assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
            }
            "socket" => socket = Some(std::os::unix::net::UnixListener::bind(&candidate).unwrap()),
            _ => unreachable!(),
        }
        let context = fixture.context();
        let request = fixture.request(&context);
        let result = new_authority().issue(
            &context,
            &request,
            fixture.effects(&request),
            10,
            20,
            &nonce(&format!("protected-{kind}")),
        );
        let failure = match result {
            Err(failure) => failure,
            Ok(_) => panic!("a protected {kind} unexpectedly issued authority"),
        };
        assert_eq!(failure.id(), AdapterErrorId::TargetUnavailable, "{kind}");
        assert!(
            CANONICAL_TEMPLATES
                .iter()
                .all(|row| !fixture.root.join(row.target_path).is_file()),
            "{kind}"
        );
        drop(socket);
    }
}

#[test]
pub(crate) fn protected_late_same_inode_write_is_detected_and_left_exactly_observable() {
    let fixture = Fixture::new("protected-late-same-inode-write-issuance");
    fixture.write("private/one.txt", b"protected-alpha\n");
    let path = fixture.root.join("private/one.txt");
    let metadata = fs::metadata(&path).unwrap();
    let inode = metadata.ino();
    let version = change_version(&path);
    let context = fixture.context();
    let request = fixture.request(&context);
    let attack_path = path.clone();
    let attacker = arm_protected_capture_barrier(
        ProtectedCaptureBoundary::PermitIssuance,
        ProtectedCapturePhase::AfterRegularRowRevalidated,
        b"private/one.txt",
        move || fs::write(attack_path, b"protected-bravo\n").unwrap(),
    );
    let result = new_authority().issue(
        &context,
        &request,
        fixture.effects(&request),
        10,
        20,
        &nonce("protected-late-same-inode-write-issuance"),
    );
    assert_protected_capture_hook_consumed_for_test();
    attacker.join().unwrap();
    let failure = match result {
        Err(failure) => failure,
        Ok(_) => panic!("a late same-inode protected write issued authority"),
    };
    assert_eq!(failure.id(), AdapterErrorId::TargetUnavailable);
    assert_late_regular_write_remains(&path, inode, b"protected-bravo\n", version);
    assert!(
        CANONICAL_TEMPLATES
            .iter()
            .all(|row| !fixture.root.join(row.target_path).is_file())
    );
}

#[test]
pub(crate) fn protected_two_file_hybrid_collect_is_rejected_without_path_sampling() {
    let fixture = Fixture::new("protected-two-file-hybrid-issuance");
    fixture.write("private/a.txt", b"file-a-before\n");
    fixture.write("private/b.txt", b"file-b-before\n");
    let first = fixture.root.join("private/a.txt");
    let second = fixture.root.join("private/b.txt");
    let first_inode = fs::metadata(&first).unwrap().ino();
    let second_inode = fs::metadata(&second).unwrap().ino();
    let context = fixture.context();
    let request = fixture.request(&context);
    let attack_first = first.clone();
    let attack_second = second.clone();
    let attacker = arm_protected_capture_barrier(
        ProtectedCaptureBoundary::PermitIssuance,
        ProtectedCapturePhase::AfterRegularRowRevalidated,
        b"private/a.txt",
        move || {
            fs::write(attack_first, b"file-a-after!\n").unwrap();
            fs::write(attack_second, b"file-b-after!\n").unwrap();
        },
    );
    let result = new_authority().issue(
        &context,
        &request,
        fixture.effects(&request),
        10,
        20,
        &nonce("protected-two-file-hybrid-issuance"),
    );
    assert_protected_capture_hook_consumed_for_test();
    attacker.join().unwrap();
    let failure = match result {
        Err(failure) => failure,
        Ok(_) => panic!("a coordinated protected hybrid issued authority"),
    };
    assert_eq!(failure.id(), AdapterErrorId::TargetUnavailable);
    assert_eq!(fs::metadata(&first).unwrap().ino(), first_inode);
    assert_eq!(fs::metadata(&second).unwrap().ino(), second_inode);
    assert_eq!(fs::read(&first).unwrap(), b"file-a-after!\n");
    assert_eq!(fs::read(&second).unwrap(), b"file-b-after!\n");
}
