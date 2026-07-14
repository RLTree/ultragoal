use super::*;

#[test]
pub(crate) fn target_complete_double_collect_rejects_two_leaf_alternating_hybrid() {
    let fixture = Fixture::new("target-two-leaf-alternating-hybrid");
    fixture.install_all();
    let first_row = &CANONICAL_TEMPLATES[0];
    let second_row = &CANONICAL_TEMPLATES[1];
    let first = fixture.root.join(first_row.target_path);
    let second = fixture.root.join(second_row.target_path);
    let first_other = same_length_other(first_row.bytes);
    let second_other = same_length_other(second_row.bytes);
    let first_version = change_version(&first);
    let second_version = change_version(&second);
    let context = fixture.context();
    let request = fixture.request(&context);
    arm_alternating_target_hybrid(
        first_row.target_path,
        second_row.target_path,
        first.clone(),
        second.clone(),
        first_row.bytes.to_vec(),
        first_other,
        second_row.bytes.to_vec(),
        second_other.clone(),
        2,
    );
    let result = new_authority().issue(
        &context,
        &request,
        fixture.effects(&request),
        10,
        20,
        &nonce("target-two-leaf-alternating-hybrid"),
    );
    assert_target_capture_hook_consumed_for_test();
    let failure = match result {
        Err(failure) => failure,
        Ok(_) => panic!("a two-leaf target hybrid unexpectedly issued authority"),
    };
    assert_eq!(failure.id(), AdapterErrorId::TargetUnavailable);
    assert_eq!(fs::read(&first).unwrap(), first_row.bytes);
    assert_eq!(fs::read(&second).unwrap(), second_row.bytes);
    assert_ne!(change_version(&first), first_version);
    assert_ne!(change_version(&second), second_version);
}

#[test]
pub(crate) fn target_change_version_rejects_same_inode_mutate_restore_aba() {
    let fixture = Fixture::new("target-same-inode-mutate-restore-aba");
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
    let attack_path = path.clone();
    let other = same_length_other(row.bytes);
    target_capture_hook_for_test(TargetCapturePhase::BeforeFinalChainRecheck, "", move || {
        fs::write(&attack_path, other).unwrap();
        fs::write(attack_path, row.bytes).unwrap();
    });
    let result = new_authority().issue(
        &context,
        &request,
        fixture.effects(&request),
        10,
        20,
        &nonce("target-same-inode-mutate-restore-aba"),
    );
    assert_target_capture_hook_consumed_for_test();
    let failure = match result {
        Err(failure) => failure,
        Ok(_) => panic!("a target mutate-restore ABA unexpectedly issued authority"),
    };
    assert_eq!(failure.id(), AdapterErrorId::TargetUnavailable);
    assert_late_regular_write_remains(&path, inode, row.bytes, version);
}

#[test]
pub(crate) fn target_complete_collect_rejects_content_length_mode_ownership_link_special_and_path_races()
 {
    assert_target_complete_collect_race("target-content-race", |target, _, _| {
        let bytes = fs::read(target).unwrap();
        fs::write(target, same_length_other(&bytes)).unwrap();
    });
    assert_target_complete_collect_race("target-length-race", |target, _, _| {
        let mut bytes = fs::read(target).unwrap();
        bytes.push(b'x');
        fs::write(target, bytes).unwrap();
    });
    assert_target_complete_collect_race("target-mode-race", |target, _, _| {
        fs::set_permissions(target, fs::Permissions::from_mode(0o600)).unwrap();
    });
    assert_target_complete_collect_race("target-ownership-race", |target, _, _| {
        let current = fs::metadata(target).unwrap().gid();
        let alternate = alternate_supplementary_gid(current);
        let encoded = std::ffi::CString::new(target.as_os_str().as_encoded_bytes()).unwrap();
        assert_eq!(
            unsafe {
                libc::chown(
                    encoded.as_ptr(),
                    !0 as libc::uid_t,
                    alternate as libc::gid_t,
                )
            },
            0
        );
        assert_eq!(fs::metadata(target).unwrap().gid(), alternate);
    });
    assert_target_complete_collect_race("target-link-race", |target, root, _| {
        fs::hard_link(target, root.join("target-hardlink-race-alias")).unwrap();
    });
    assert_target_complete_collect_race("target-special-race", |target, _, container| {
        fs::rename(target, container.join("target-special-race-original")).unwrap();
        let encoded = std::ffi::CString::new(target.as_os_str().as_encoded_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(encoded.as_ptr(), 0o600) }, 0);
    });
    assert_target_complete_collect_race("target-path-race", |target, _, _| {
        fs::rename(target, target.with_file_name("Agents.md")).unwrap();
    });
}

#[test]
pub(crate) fn mutation_after_postflight_cannot_cross_the_final_green_ancestor_boundary() {
    assert_final_green_ancestor_drift(
        "unique-ancestor-replacement-final-green",
        "validation_artifacts/harness",
        |path, displaced| replace_directory_preserving_children(&path, &displaced),
    );
    assert_final_green_ancestor_drift(
        "shared-ancestor-replacement-final-green",
        "agent-standards",
        |path, displaced| replace_directory_preserving_children(&path, &displaced),
    );
    assert_final_green_ancestor_drift("ancestor-mode-final-green", ".harness", |path, _| {
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
    });
    assert_final_green_ancestor_drift(
        "ancestor-symlink-final-green",
        ".codex/environments",
        |path, displaced| {
            fs::rename(&path, &displaced).unwrap();
            symlink(&displaced, path).unwrap();
        },
    );
    assert_final_green_ancestor_drift(
        "ancestor-special-final-green",
        "validation_artifacts/harness",
        |path, displaced| replace_directory_with_fifo(&path, &displaced),
    );
    assert_final_green_ancestor_drift(
        "ancestor-case-alias-final-green",
        "agent-standards",
        |path, _| {
            let alias = path.parent().unwrap().join("Agent-Standards");
            fs::rename(path, alias).unwrap();
        },
    );
}
