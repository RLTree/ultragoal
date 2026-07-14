use super::*;

#[test]
pub(crate) fn positive_supported_host_fresh_setup_and_repeat_use_are_exact_and_idempotent() {
    let fixture = Fixture::new("positive-fresh-repeat");
    let context = fixture.context();
    let inspection = assert_zero_write(&fixture, || inspect_target(&context).unwrap());
    let record = assert_zero_write(&fixture, || plan_target(&context).unwrap());
    assert_eq!(inspection.classification(), "fresh");
    assert_eq!(record.mutation_count(), CANONICAL_TEMPLATES.len());

    let request = fixture.request(&context);
    let outcome = apply_once(&fixture, &context, request, "positive-fresh");
    assert_eq!(outcome.status(), "applied");
    assert_eq!(outcome.mutation_count(), CANONICAL_TEMPLATES.len());
    for row in CANONICAL_TEMPLATES {
        let path = fixture.root.join(row.target_path);
        assert_eq!(fs::read(&path).unwrap(), row.bytes);
        assert_eq!(
            fs::metadata(path).unwrap().permissions().mode() & 0o7777,
            row.unix_mode
        );
    }

    let repeated_context = fixture.context();
    let repeated_record = assert_zero_write(&fixture, || plan_target(&repeated_context).unwrap());
    assert_eq!(repeated_record.mutation_count(), 0);
    let repeated = fixture.request(&repeated_context);
    let repeated_outcome = apply_once(
        &fixture,
        &repeated_context,
        repeated,
        "positive-repeat-idempotent",
    );
    assert_eq!(repeated_outcome.status(), "idempotent");
    assert_eq!(repeated_outcome.mutation_count(), 0);
    let verification = assert_zero_write(&fixture, || verify_target(&fixture.context()).unwrap());
    assert!(verification.idempotent());
    assert_eq!(verification.matched_files(), CANONICAL_TEMPLATES.len());
}

#[test]
pub(crate) fn positive_supported_host_partial_retrofit_preserves_dirty_user_state() {
    let fixture = Fixture::new("positive-partial-dirty");
    fixture.write("README.md", b"tracked baseline\n");
    fixture.commit_all("baseline");
    fixture.write("README.md", b"tracked dirty user edit\n");
    fixture.write("private/untracked-canary.txt", b"private dirty bytes\n");
    fixture.write_template("AGENTS.md");
    let user_diff = git(&fixture.root, &["diff", "--", "README.md"]).stdout;
    let tracked = fs::read(fixture.root.join("README.md")).unwrap();
    let untracked = fs::read(fixture.root.join("private/untracked-canary.txt")).unwrap();

    let context = fixture.context();
    assert!(context.candidate().dirty);
    let inspection = assert_zero_write(&fixture, || inspect_target(&context).unwrap());
    let record = assert_zero_write(&fixture, || plan_target(&context).unwrap());
    assert_eq!(inspection.classification(), "partial");
    assert_eq!(record.mutation_count(), CANONICAL_TEMPLATES.len() - 1);
    let request = fixture.request(&context);
    let outcome = apply_once(&fixture, &context, request, "positive-partial-dirty");
    assert_eq!(outcome.status(), "applied");
    assert_eq!(fs::read(fixture.root.join("README.md")).unwrap(), tracked);
    assert_eq!(
        fs::read(fixture.root.join("private/untracked-canary.txt")).unwrap(),
        untracked
    );
    assert_eq!(
        git(&fixture.root, &["diff", "--", "README.md"]).stdout,
        user_diff
    );
    assert!(verify_target(&fixture.context()).unwrap().idempotent());
}

#[test]
pub(crate) fn negative_conflict_and_settled_request_replay_refuse_without_effect() {
    let conflict = Fixture::new("negative-conflict");
    conflict.write("AGENTS.md", b"user-owned repository instructions\n");
    conflict.commit_all("user authority");
    let context = conflict.context();
    let before_tree = snapshot(&conflict.root);
    let before_status = git_status(&conflict.root);
    let inspection = inspect_target(&context).unwrap();
    let record = plan_target(&context).unwrap();
    assert_eq!(inspection.classification(), "conflicting");
    assert_eq!(record.conflict_count(), 1);
    let failure = prepare_apply_request(
        &context,
        &record.to_machine_bytes().unwrap(),
        record.plan_sha256(),
    )
    .err()
    .unwrap();
    assert_eq!(failure.id(), AdapterErrorId::PlanConflict);
    assert_eq!(snapshot(&conflict.root), before_tree);
    assert_eq!(git_status(&conflict.root), before_status);

    let replay = Fixture::new("negative-replay");
    replay.install_all_direct();
    let context = replay.context();
    let request = replay.request(&context);
    let duplicate_request = request.duplicate_for_test();
    let (permit, lease) = issue(
        &context,
        &request,
        replay.effects(&request),
        "negative-replay",
    );
    let (duplicate_permit, duplicate_lease) = duplicate_authorization_for_test(
        &permit,
        &duplicate_request,
        replay.effects(&duplicate_request),
    );
    let first = apply_ok(apply_with_root_permit(
        &context,
        request,
        Some(permit),
        Some(lease),
        10,
    ));
    assert_eq!(first.status(), "idempotent");
    let replay_tree = snapshot(&replay.root);
    let replay_status = git_status(&replay.root);
    let failure = apply_failure(apply_with_root_permit(
        &context,
        duplicate_request,
        Some(duplicate_permit),
        Some(duplicate_lease),
        10,
    ));
    assert_eq!(failure.error().id(), AdapterErrorId::ApplyPermitReplayed);
    assert_eq!(snapshot(&replay.root), replay_tree);
    assert_eq!(git_status(&replay.root), replay_status);
}

#[test]
pub(crate) fn mutation_failure_rolls_back_exactly_and_a_new_request_recovers() {
    let fixture = Fixture::new("mutation-rollback-recovery");
    fixture.write("private/user.txt", b"preserve me exactly\n");
    let context = fixture.context();
    let request = fixture.request(&context);
    let before_tree = snapshot(&fixture.root);
    let before_status = git_status(&fixture.root);
    let mut effects = fixture.effects(&request);
    effects.fail_on_calls([2]);
    let (permit, lease) = issue(&context, &request, effects, "mutation-rollback");
    let failure = apply_failure(apply_with_root_permit(
        &context,
        request,
        Some(permit),
        Some(lease),
        10,
    ));
    assert_eq!(failure.error().id(), AdapterErrorId::ApplyRolledBack);
    assert!(failure.effect_started());
    assert!(failure.rollback_complete());
    assert_eq!(snapshot(&fixture.root), before_tree);
    assert_eq!(git_status(&fixture.root), before_status);

    let recovery_context = fixture.context();
    let recovery = fixture.request(&recovery_context);
    let outcome = apply_once(
        &fixture,
        &recovery_context,
        recovery,
        "mutation-rollback-recovery",
    );
    assert_eq!(outcome.status(), "applied");
    assert!(verify_target(&fixture.context()).unwrap().idempotent());
}
