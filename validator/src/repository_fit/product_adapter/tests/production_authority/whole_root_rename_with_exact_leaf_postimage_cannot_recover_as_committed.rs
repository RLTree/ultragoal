use super::*;

#[test]
pub(crate) fn whole_root_rename_with_exact_leaf_postimage_cannot_recover_as_committed() {
    let fixture = Fixture::new("process-crash-after-effect-root-rename");
    let prepared_root_binding = observed_root_binding(&fixture.root);
    let original_root = fs::symlink_metadata(&fixture.root).unwrap();
    let _intent = run_authority_crash(&fixture, "crash-after", "crash-after-root-rename-owner", 88);
    let exact_postimage = snapshot(&fixture.root);
    let exact_post_status = git_status(&fixture.root);

    let relocated_root = fixture.container.join("relocated-repo");
    fs::rename(&fixture.root, &relocated_root).unwrap();
    let relocated_metadata = fs::symlink_metadata(&relocated_root).unwrap();
    assert_eq!(relocated_metadata.dev(), original_root.dev());
    assert_eq!(relocated_metadata.ino(), original_root.ino());
    assert_eq!(snapshot(&relocated_root), exact_postimage);
    assert_eq!(git_status(&relocated_root), exact_post_status);
    let relocated_root_binding = observed_root_binding(&relocated_root);
    assert_ne!(relocated_root_binding, prepared_root_binding);

    let before_recovery = snapshot(&relocated_root);
    let before_recovery_status = git_status(&relocated_root);
    let recovered = run_authority_scenario_at_root(
        &fixture,
        &relocated_root,
        "recover-expired",
        "crash-after-root-rename-owner",
    );
    assert_eq!(recovered["status"], "ambiguous");
    assert_eq!(recovered["adapter_error_id"], "apply_outcome_ambiguous");
    assert_eq!(recovered["ledger_state"], "ambiguous");
    assert_eq!(recovered["effect_started"], true);
    assert_eq!(recovered["effect"], "none");
    assert_eq!(snapshot(&relocated_root), before_recovery);
    assert_eq!(git_status(&relocated_root), before_recovery_status);
    assert!(!fixture.root.exists());

    let terminal = fs::read_to_string(fixture.store.root.join("authority-ledger.json")).unwrap();
    assert_eq!(terminal.matches("\"state\":\"reserved\"").count(), 1);
    assert_eq!(terminal.matches("\"state\":\"effect_started\"").count(), 1);
    assert_eq!(terminal.matches("\"state\":\"ambiguous\"").count(), 1);
    assert!(!terminal.contains("committed"));
}

#[test]
pub(crate) fn abrupt_process_exit_with_exact_existing_ancestor_postimage_recovers_as_committed() {
    let fixture = Fixture::new("process-crash-existing-ancestor-exact");
    let ancestor = create_existing_codex_ancestor(&fixture);
    let identity = fs::symlink_metadata(&ancestor).unwrap().ino();
    let _intent = run_authority_crash(
        &fixture,
        "crash-after",
        "crash-existing-ancestor-exact-owner",
        88,
    );
    assert_eq!(fs::symlink_metadata(&ancestor).unwrap().ino(), identity);

    let recovered = run_authority_scenario(
        &fixture,
        "recover-expired",
        "crash-existing-ancestor-exact-owner",
    );
    assert_eq!(recovered["status"], "recovered");
    assert_eq!(recovered["adapter_error_id"], serde_json::Value::Null);
    assert_eq!(recovered["ledger_state"], "committed");
    assert_eq!(recovered["effect_started"], true);
}

#[test]
pub(crate) fn existing_ancestor_replacement_with_identical_leaf_postimage_cannot_recover_as_committed()
 {
    let fixture = Fixture::new("process-crash-existing-ancestor-replaced");
    let ancestor = create_existing_codex_ancestor(&fixture);
    let original_identity = fs::symlink_metadata(&ancestor).unwrap().ino();
    let _intent = run_authority_crash(
        &fixture,
        "crash-after",
        "crash-existing-ancestor-replaced-owner",
        88,
    );
    let expected_leaves = CANONICAL_TEMPLATES
        .iter()
        .filter(|row| row.target_path.starts_with(".codex/"))
        .map(|row| {
            (
                row.target_path,
                fs::read(fixture.root.join(row.target_path)).unwrap(),
                fs::symlink_metadata(fixture.root.join(row.target_path))
                    .unwrap()
                    .mode()
                    & 0o7777,
            )
        })
        .collect::<Vec<_>>();
    let quarantine = fixture.container.join("original-codex-ancestor");
    replace_directory_preserving_children(&ancestor, &quarantine);
    assert_ne!(
        fs::symlink_metadata(&ancestor).unwrap().ino(),
        original_identity
    );
    for (path, bytes, mode) in expected_leaves {
        assert_eq!(fs::read(fixture.root.join(path)).unwrap(), bytes);
        assert_eq!(
            fs::symlink_metadata(fixture.root.join(path))
                .unwrap()
                .mode()
                & 0o7777,
            mode
        );
    }

    assert_expired_recovery_is_ambiguous(&fixture, "crash-existing-ancestor-replaced-owner");
}

#[test]
pub(crate) fn existing_ancestor_mode_drift_cannot_recover_as_committed() {
    let fixture = Fixture::new("process-crash-existing-ancestor-mode-drift");
    let ancestor = create_existing_codex_ancestor(&fixture);
    let _intent = run_authority_crash(
        &fixture,
        "crash-after",
        "crash-existing-ancestor-mode-owner",
        88,
    );
    fs::set_permissions(&ancestor, fs::Permissions::from_mode(0o700)).unwrap();
    assert_expired_recovery_is_ambiguous(&fixture, "crash-existing-ancestor-mode-owner");
}

#[test]
pub(crate) fn existing_ancestor_group_drift_cannot_recover_as_committed_when_representable() {
    let fixture = Fixture::new("process-crash-existing-ancestor-group-drift");
    let ancestor = create_existing_codex_ancestor(&fixture);
    let _intent = run_authority_crash(
        &fixture,
        "crash-after",
        "crash-existing-ancestor-group-owner",
        88,
    );
    if change_group_if_representable(&ancestor) {
        assert_expired_recovery_is_ambiguous(&fixture, "crash-existing-ancestor-group-owner");
    } else {
        eprintln!("managed ancestor group drift is not representable for this host user");
    }
}

#[test]
pub(crate) fn existing_ancestor_symlink_containment_alias_cannot_recover_as_committed() {
    let fixture = Fixture::new("process-crash-existing-ancestor-alias");
    let ancestor = create_existing_codex_ancestor(&fixture);
    let _intent = run_authority_crash(
        &fixture,
        "crash-after",
        "crash-existing-ancestor-alias-owner",
        88,
    );
    let quarantine = fixture.container.join("aliased-codex-ancestor");
    fs::rename(&ancestor, &quarantine).unwrap();
    symlink(&quarantine, &ancestor).unwrap();
    for row in CANONICAL_TEMPLATES
        .iter()
        .filter(|row| row.target_path.starts_with(".codex/"))
    {
        assert_eq!(
            fs::read(fixture.root.join(row.target_path)).unwrap(),
            row.bytes
        );
    }

    assert_expired_recovery_is_ambiguous(&fixture, "crash-existing-ancestor-alias-owner");
}
