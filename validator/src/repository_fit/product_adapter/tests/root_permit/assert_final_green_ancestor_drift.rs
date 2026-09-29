use super::*;

pub(crate) fn assert_final_green_ancestor_drift(
    label: &str,
    ancestor: &str,
    mutate: impl FnOnce(PathBuf, PathBuf) + 'static,
) {
    let fixture = Fixture::new(label);
    let path = fixture.root.join(ancestor);
    fs::create_dir_all(&path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    let context = fixture.context();
    let request = fixture.request(&context);
    assert!(!request.plan.mutations.is_empty());
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            fixture.effects(&request),
            10,
            20,
            &nonce(label),
        )
        .unwrap();
    let displaced = fixture.container.join(format!("{label}-displaced"));
    before_final_green_observation_for_test(move || mutate(path, displaced));
    let failure = expect_apply_failure(apply_with_root_permit(
        &context,
        request,
        Some(permit),
        Some(lease),
        10,
    ));
    assert_eq!(failure.error().id(), AdapterErrorId::ApplyOutcomeAmbiguous);
    assert!(failure.effect_started(), "{label}");
    assert!(!failure.rollback_complete(), "{label}");
}

#[test]
pub(crate) fn fresh_partial_dirty_and_idempotent_apply_are_exact_and_causal() {
    let fresh = Fixture::new("fresh-and-repeat");
    let context = fresh.context();
    let request = fresh.request(&context);
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            fresh.effects(&request),
            10,
            20,
            &nonce("fresh"),
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
    assert_eq!(outcome.mutation_count(), CANONICAL_TEMPLATES.len() + 1);
    assert!(outcome.outcome_id().starts_with("sha256:"));
    assert!(verify_target(&fresh.context()).unwrap().idempotent());

    let repeated_context = fresh.context();
    let repeated = fresh.request(&repeated_context);
    assert!(repeated.plan.mutations.is_empty());
    let repeated_authority = new_authority();
    let (permit, lease) = repeated_authority
        .issue(
            &repeated_context,
            &repeated,
            fresh.effects(&repeated),
            30,
            40,
            &nonce("repeat"),
        )
        .unwrap();
    let repeated_outcome = expect_apply_ok(apply_with_root_permit(
        &repeated_context,
        repeated,
        Some(permit),
        Some(lease),
        30,
    ));
    assert_eq!(repeated_outcome.status(), "idempotent");
    assert_eq!(repeated_outcome.mutation_count(), 0);

    let partial = Fixture::new("partial-dirty");
    partial.write_template("AGENTS.md");
    partial.write(
        "private/unrelated.txt",
        b"preserve this exact dirty state\n",
    );
    let unrelated = fs::read(partial.root.join("private/unrelated.txt")).unwrap();
    let context = partial.context();
    assert!(context.candidate().dirty);
    let request = partial.request(&context);
    assert_eq!(request.plan.mutations.len(), CANONICAL_TEMPLATES.len() - 1);
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            partial.effects(&request),
            50,
            60,
            &nonce("partial-dirty"),
        )
        .unwrap();
    let outcome = expect_apply_ok(apply_with_root_permit(
        &context,
        request,
        Some(permit),
        Some(lease),
        50,
    ));
    assert_eq!(outcome.status(), "applied");
    assert_eq!(
        fs::read(partial.root.join("private/unrelated.txt")).unwrap(),
        unrelated
    );
}

#[test]
pub(crate) fn missing_managed_ancestors_are_created_with_exact_root_ownership_and_mode() {
    let fixture = Fixture::new("created-managed-ancestors");
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
            &nonce("created-managed-ancestors"),
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
    let root = fs::symlink_metadata(&fixture.root).unwrap();
    let ancestors = catalog_ancestor_paths();
    assert!(ancestors.len() >= 8);
    for ancestor in ancestors {
        let metadata = fs::symlink_metadata(fixture.root.join(&ancestor)).unwrap();
        assert!(metadata.is_dir(), "{}", ancestor.display());
        assert_eq!(metadata.dev(), root.dev(), "{}", ancestor.display());
        assert_eq!(metadata.uid(), root.uid(), "{}", ancestor.display());
        assert_eq!(metadata.gid(), root.gid(), "{}", ancestor.display());
        assert_eq!(metadata.mode() & 0o7777, 0o755, "{}", ancestor.display());
    }
    assert!(verify_target(&fixture.context()).unwrap().idempotent());
}
