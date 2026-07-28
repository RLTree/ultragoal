use super::*;

#[test]
pub(crate) fn descriptor_capture_rejects_parent_leaf_and_missing_boundary_hybrids_without_effects()
{
    let parent = Fixture::new("descriptor-parent-hybrid");
    parent.install_all();
    let context = parent.context();
    let request = parent.request(&context);
    let before = CANONICAL_TEMPLATES
        .iter()
        .map(|row| {
            (
                row.target_path,
                fs::read(parent.root.join(row.target_path)).unwrap(),
            )
        })
        .collect::<Vec<_>>();
    let active = parent.root.join("agent-standards");
    let displaced = parent.container.join("descriptor-parent-a");
    let attacker = arm_capture_barrier(
        TargetCapturePhase::AfterParentHeldBeforeDescend,
        "agent-standards",
        move || replace_directory_preserving_children(&active, &displaced),
    );
    let failure = match new_authority().issue(
        &context,
        &request,
        parent.effects(&request),
        10,
        20,
        &nonce("descriptor-parent-hybrid"),
    ) {
        Err(failure) => failure,
        Ok(_) => panic!("a parent A/B hybrid unexpectedly issued authority"),
    };
    attacker.join().unwrap();
    assert_target_capture_hook_consumed_for_test();
    assert_eq!(failure.id(), AdapterErrorId::TargetUnavailable);
    for (path, bytes) in before {
        assert_eq!(fs::read(parent.root.join(path)).unwrap(), bytes, "{path}");
    }

    let named = Fixture::new("descriptor-named-parent-hybrid");
    named.install_all();
    let context = named.context();
    let request = named.request(&context);
    let active = named.root.join("agent-standards");
    let displaced = named.container.join("descriptor-named-parent-a");
    let attacker = arm_capture_barrier(
        TargetCapturePhase::AfterNamedBeforeOpen,
        "agent-standards",
        move || replace_directory_preserving_children(&active, &displaced),
    );
    let failure = match new_authority().issue(
        &context,
        &request,
        named.effects(&request),
        10,
        20,
        &nonce("descriptor-named-parent-hybrid"),
    ) {
        Err(failure) => failure,
        Ok(_) => panic!("a named-to-open parent hybrid unexpectedly issued authority"),
    };
    attacker.join().unwrap();
    assert_target_capture_hook_consumed_for_test();
    assert_eq!(failure.id(), AdapterErrorId::TargetUnavailable);

    let leaf = Fixture::new("descriptor-leaf-hybrid");
    leaf.install_all();
    let context = leaf.context();
    let request = leaf.request(&context);
    let active = leaf.root.join("AGENTS.md");
    let displaced = leaf.container.join("descriptor-leaf-a");
    let bytes = fs::read(&active).unwrap();
    let attacker = arm_capture_barrier(
        TargetCapturePhase::AfterLeafHeldBeforeRead,
        "AGENTS.md",
        move || {
            fs::rename(&active, displaced).unwrap();
            fs::write(&active, &bytes).unwrap();
            fs::set_permissions(&active, fs::Permissions::from_mode(0o644)).unwrap();
        },
    );
    let failure = match new_authority().issue(
        &context,
        &request,
        leaf.effects(&request),
        10,
        20,
        &nonce("descriptor-leaf-hybrid"),
    ) {
        Err(failure) => failure,
        Ok(_) => panic!("a leaf A/B hybrid unexpectedly issued authority"),
    };
    attacker.join().unwrap();
    assert_target_capture_hook_consumed_for_test();
    assert_eq!(failure.id(), AdapterErrorId::TargetUnavailable);

    let race_fixture = Fixture::new("descriptor-shared-ancestor-race");
    race_fixture.install_all();
    let context = race_fixture.context();
    let request = race_fixture.request(&context);
    let active = race_fixture.root.join("agent-standards");
    let displaced = race_fixture.container.join("descriptor-shared-a");
    let attacker =
        arm_capture_barrier(TargetCapturePhase::BeforeFinalChainRecheck, "", move || {
            replace_directory_preserving_children(&active, &displaced)
        });
    let failure = match new_authority().issue(
        &context,
        &request,
        race_fixture.effects(&request),
        10,
        20,
        &nonce("descriptor-shared-ancestor-race"),
    ) {
        Err(failure) => failure,
        Ok(_) => panic!("a shared-ancestor final-recheck race unexpectedly issued authority"),
    };
    attacker.join().unwrap();
    assert_target_capture_hook_consumed_for_test();
    assert_eq!(failure.id(), AdapterErrorId::TargetUnavailable);

    let aliased = Fixture::new("descriptor-case-alias-race");
    aliased.install_all();
    let context = aliased.context();
    let request = aliased.request(&context);
    let active = aliased.root.join("agent-standards");
    let alias = aliased.root.join("Agent-Standards");
    let attacker = arm_capture_barrier(
        TargetCapturePhase::AfterNamedBeforeOpen,
        "agent-standards",
        move || fs::rename(active, alias).unwrap(),
    );
    let failure = match new_authority().issue(
        &context,
        &request,
        aliased.effects(&request),
        10,
        20,
        &nonce("descriptor-case-alias-race"),
    ) {
        Err(failure) => failure,
        Ok(_) => panic!("a within-capture case alias unexpectedly issued authority"),
    };
    attacker.join().unwrap();
    assert_target_capture_hook_consumed_for_test();
    assert_eq!(failure.id(), AdapterErrorId::TargetUnavailable);

    let missing = Fixture::new("descriptor-missing-race");
    let context = missing.context();
    let request = missing.request(&context);
    let created = missing.root.join("agent-standards");
    let attacker = arm_capture_barrier(
        TargetCapturePhase::AfterMissingBeforeRecheck,
        "agent-standards",
        move || {
            fs::create_dir(&created).unwrap();
            fs::set_permissions(created, fs::Permissions::from_mode(0o755)).unwrap();
        },
    );
    let failure = match new_authority().issue(
        &context,
        &request,
        missing.effects(&request),
        10,
        20,
        &nonce("descriptor-missing-race"),
    ) {
        Err(failure) => failure,
        Ok(_) => panic!("a missing-boundary race unexpectedly issued authority"),
    };
    attacker.join().unwrap();
    assert_target_capture_hook_consumed_for_test();
    assert_eq!(failure.id(), AdapterErrorId::TargetUnavailable);
    assert!(
        CANONICAL_TEMPLATES
            .iter()
            .all(|row| !missing.root.join(row.target_path).is_file())
    );
}
