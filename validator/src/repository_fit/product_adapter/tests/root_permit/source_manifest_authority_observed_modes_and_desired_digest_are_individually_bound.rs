use super::*;

#[test]
pub(crate) fn source_manifest_authority_observed_modes_and_desired_digest_are_individually_bound() {
    assert_request_tamper_rejected("manifest", |request| {
        request.authority.manifest_sha256 = digest(b"substituted source manifest");
    });
    assert_request_tamper_rejected("authority", |request| {
        request.authority.authority_sha256 = digest(b"substituted source authority");
    });
    assert_request_tamper_rejected("observed-mode", |request| {
        let path = request.plan.mutations[0].path.as_str().to_owned();
        request.observed_modes.insert(path, Some(0o777));
    });
    assert_request_tamper_rejected("desired-state-digest", |request| {
        request.desired.state_sha256 = digest(b"substituted desired state digest");
    });
}

#[test]
pub(crate) fn same_content_target_replacement_permissions_git_state_and_root_replacement_are_stale()
{
    let target = Fixture::new("target-replacement");
    target.write_template("AGENTS.md");
    let context = target.context();
    let request = target.request(&context);
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            target.effects(&request),
            10,
            20,
            &nonce("target-replacement"),
        )
        .unwrap();
    let original = target.root.join("AGENTS.md");
    let replacement = target.root.join("replacement.tmp");
    fs::write(&replacement, fs::read(&original).unwrap()).unwrap();
    fs::set_permissions(&replacement, fs::Permissions::from_mode(0o644)).unwrap();
    fs::rename(&replacement, &original).unwrap();
    let failure = expect_apply_failure(apply_with_root_permit(
        &context,
        request,
        Some(permit),
        Some(lease),
        10,
    ));
    assert!(!failure.effect_started());

    let permissions = Fixture::new("permission-stale");
    permissions.write_template("AGENTS.md");
    let context = permissions.context();
    let request = permissions.request(&context);
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            permissions.effects(&request),
            10,
            20,
            &nonce("permission-stale"),
        )
        .unwrap();
    fs::set_permissions(
        permissions.root.join("AGENTS.md"),
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    let failure = expect_apply_failure(apply_with_root_permit(
        &context,
        request,
        Some(permit),
        Some(lease),
        10,
    ));
    assert!(!failure.effect_started());

    let git_mutation = Fixture::new("git-state-stale");
    let context = git_mutation.context();
    let request = git_mutation.request(&context);
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            git_mutation.effects(&request),
            10,
            20,
            &nonce("git-state-stale"),
        )
        .unwrap();
    let config = git_mutation.root.join(".git/config");
    let mut bytes = fs::read(&config).unwrap();
    bytes.extend_from_slice(b"\n[fit-test]\n\tchanged = true\n");
    fs::write(&config, bytes).unwrap();
    let failure = expect_apply_failure(apply_with_root_permit(
        &context,
        request,
        Some(permit),
        Some(lease),
        10,
    ));
    assert!(!failure.effect_started());

    let root_swap = Fixture::new("root-replacement");
    let context = root_swap.context();
    let request = root_swap.request(&context);
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            root_swap.effects(&request),
            10,
            20,
            &nonce("root-replacement"),
        )
        .unwrap();
    let displaced = root_swap.container.join("displaced-repo");
    fs::rename(&root_swap.root, &displaced).unwrap();
    fs::create_dir(&root_swap.root).unwrap();
    git(&root_swap.root, &["init", "--quiet"]);
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
pub(crate) fn every_existing_managed_ancestor_is_bound_before_effect() {
    assert_pre_effect_ancestor_drift(
        "unique-ancestor-replacement-before-effect",
        "validation_artifacts/harness",
        |fixture, path| {
            replace_directory_preserving_children(
                path,
                &fixture.container.join("unique-ancestor-original"),
            );
        },
    );
    assert_pre_effect_ancestor_drift(
        "shared-ancestor-replacement-before-effect",
        "agent-standards",
        |fixture, path| {
            replace_directory_preserving_children(
                path,
                &fixture.container.join("shared-ancestor-original"),
            );
        },
    );
    assert_pre_effect_ancestor_drift("ancestor-mode-before-effect", ".harness", |_, path| {
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
    });
    assert_pre_effect_ancestor_drift(
        "ancestor-symlink-before-effect",
        ".codex/environments",
        |fixture, path| {
            let displaced = fixture.container.join("ancestor-symlink-original");
            fs::rename(path, &displaced).unwrap();
            symlink(&displaced, path).unwrap();
        },
    );
    assert_pre_effect_ancestor_drift(
        "ancestor-special-before-effect",
        "validation_artifacts/harness",
        |fixture, path| {
            replace_directory_with_fifo(path, &fixture.container.join("ancestor-special-original"));
        },
    );
    assert_pre_effect_ancestor_drift(
        "ancestor-case-alias-before-effect",
        "agent-standards",
        |_, path| {
            fs::rename(path, path.parent().unwrap().join("Agent-Standards")).unwrap();
        },
    );
}
