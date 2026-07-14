use super::*;

#[test]
pub(crate) fn fresh_inspect_and_plan_are_deterministic_zero_write() {
    let fixture = Fixture::new("fresh");
    let context = fixture.context();
    let (inspect, first, second) = assert_zero_write(&fixture, || {
        (
            inspect_target(&context).unwrap(),
            plan_target(&context).unwrap(),
            plan_target(&context).unwrap(),
        )
    });
    assert_eq!(inspect.classification(), "fresh");
    assert!(inspect.compatible());
    assert_eq!(first, second);
    assert_eq!(first.mutation_count(), CANONICAL_TEMPLATES.len());
    assert_eq!(first.conflict_count(), 0);
    let machine = first.to_machine_bytes().unwrap();
    assert!(!String::from_utf8_lossy(&machine).contains(&*fixture.root.to_string_lossy()));
}

#[test]
pub(crate) fn partial_retrofit_is_compatible_and_preserves_unrelated_dirty_state() {
    let fixture = Fixture::new("partial");
    fixture.write_template("AGENTS.md");
    fixture.write("user-notes/private.txt", b"preserve exactly\n");
    let context = fixture.context();
    assert!(context.candidate().dirty);
    let before = fs::read(fixture.root.join("user-notes/private.txt")).unwrap();
    let inspect = assert_zero_write(&fixture, || inspect_target(&context).unwrap());
    let plan = assert_zero_write(&fixture, || plan_target(&context).unwrap());
    assert_eq!(inspect.classification(), "partial");
    assert!(inspect.compatible());
    assert_eq!(plan.mutation_count(), CANONICAL_TEMPLATES.len() - 1);
    assert_eq!(plan.conflict_count(), 0);
    assert_eq!(
        fs::read(fixture.root.join("user-notes/private.txt")).unwrap(),
        before
    );
}

#[test]
pub(crate) fn conflicting_authority_is_causal_and_never_prepared_for_apply() {
    let fixture = Fixture::new("conflict");
    fixture.write("AGENTS.md", b"user-owned authority\n");
    let context = fixture.context();
    let inspect = assert_zero_write(&fixture, || inspect_target(&context).unwrap());
    let record = assert_zero_write(&fixture, || plan_target(&context).unwrap());
    assert_eq!(inspect.classification(), "conflicting");
    assert!(!inspect.compatible());
    assert_eq!(record.conflict_count(), 1);
    let failure = assert_zero_write(&fixture, || {
        crate::repository_fit::product_adapter::prepare_apply_request(
            &context,
            &record.to_machine_bytes().unwrap(),
            record.plan_sha256(),
        )
        .err()
        .unwrap()
    });
    assert_eq!(
        failure.id(),
        crate::repository_fit::product_adapter::AdapterErrorId::PlanConflict
    );
}

#[test]
pub(crate) fn dirty_repository_identity_is_projected_without_blocking_a_safe_plan() {
    let fixture = Fixture::new("dirty");
    fixture.write("unrelated.txt", b"dirty user bytes\n");
    let context = fixture.context();
    assert!(context.candidate().dirty);
    let record = assert_zero_write(&fixture, || plan_target(&context).unwrap());
    assert!(record.target.candidate.dirty);
    assert_eq!(record.conflict_count(), 0);
    assert_eq!(record.mutation_count(), CANONICAL_TEMPLATES.len());
}

#[test]
pub(crate) fn accepted_plan_preparation_is_zero_write_and_emits_only_one_opaque_request() {
    let fixture = Fixture::new("opaque-preparation");
    let context = fixture.context();
    let record = plan_target(&context).unwrap();
    let bytes = record.to_machine_bytes().unwrap();
    let prepared = assert_zero_write(&fixture, || {
        crate::repository_fit::product_adapter::prepare_apply_request(
            &context,
            &bytes,
            record.plan_sha256(),
        )
        .unwrap()
    });
    assert_eq!(prepared.request().context_id(), context.context_id());
    assert_eq!(prepared.request().plan_sha256(), record.plan_sha256());
    assert_eq!(
        prepared.request().request_id(),
        prepared.projection().request_id
    );
    assert_eq!(prepared.projection().effect, "workspace_write_not_executed");
    assert_eq!(prepared.projection().claim_effect, "none");
    assert_eq!(
        prepared.projection().mutation_count,
        CANONICAL_TEMPLATES.len()
    );
}

#[test]
pub(crate) fn accepted_apply_uses_atomic_local_effects_and_repeat_use_is_idempotent() {
    let fixture = Fixture::new("apply-repeat");
    let context = fixture.context();
    let prepared = fixture.plan(&context);
    let verification = execute(&fixture, prepared).unwrap();
    assert_eq!(verification.matched_files(), CANONICAL_TEMPLATES.len());
    assert!(verification.idempotent());
    for row in crate::repository_fit::product_adapter::catalog::CANONICAL_TEMPLATES {
        let path = fixture.root.join(row.target_path);
        assert_eq!(fs::read(&path).unwrap(), row.bytes);
        assert_eq!(fs::metadata(path).unwrap().mode() & 0o7777, row.unix_mode);
    }
    let current = fixture.context();
    let inspect = assert_zero_write(&fixture, || inspect_target(&current).unwrap());
    let plan = assert_zero_write(&fixture, || plan_target(&current).unwrap());
    let verified = assert_zero_write(&fixture, || verify_target(&current).unwrap());
    assert_eq!(inspect.classification(), "already_fitted");
    assert_eq!(plan.mutation_count(), 0);
    assert_eq!(plan.conflict_count(), 0);
    assert!(verified.idempotent());
    assert_eq!(verified.matched_files(), CANONICAL_TEMPLATES.len());
}

#[test]
pub(crate) fn matching_bytes_with_wrong_mode_produce_one_causal_mode_only_mutation() {
    let fixture = Fixture::new("mode-only-drift");
    let fresh = fixture.context();
    execute(&fixture, fixture.plan(&fresh)).unwrap();
    fs::set_permissions(
        fixture.root.join("scripts/check"),
        fs::Permissions::from_mode(0o644),
    )
    .unwrap();
    let drifted = fixture.context();
    let inspection = inspect_target(&drifted).unwrap();
    let record = plan_target(&drifted).unwrap();
    assert_eq!(inspection.classification(), "partial");
    assert_eq!(record.mutation_count(), 1);
    let mutation = &record.plan.mutations[0];
    assert_eq!(mutation.path, "scripts/check");
    assert_eq!(mutation.replacement_unix_mode, 0o755);
    assert_eq!(mutation.rollback.unix_mode, Some(0o644));
    assert_eq!(
        mutation.replacement_sha256,
        mutation.rollback.sha256.as_ref().unwrap().as_str()
    );
    execute(&fixture, fixture.plan(&drifted)).unwrap();
    assert_eq!(
        fs::metadata(fixture.root.join("scripts/check"))
            .unwrap()
            .mode()
            & 0o7777,
        0o755
    );
    assert!(verify_target(&fixture.context()).unwrap().idempotent());
}

#[test]
pub(crate) fn authoritative_mode_rejects_and_repairs_each_special_permission_bit() {
    for drifted_mode in [0o4755, 0o2755, 0o1755] {
        let fixture = Fixture::new(&format!("special-mode-{drifted_mode:o}"));
        fixture.install_all_direct();
        let target = fixture.root.join("scripts/check");
        std::os::unix::fs::chown(&target, None, Some(unsafe { libc::getgid() })).unwrap();
        fs::set_permissions(&target, fs::Permissions::from_mode(drifted_mode)).unwrap();
        assert_eq!(
            fs::metadata(&target).unwrap().mode() & 0o7777,
            drifted_mode,
            "control must install the requested special mode"
        );

        let drifted = fixture.context();
        let inspection = inspect_target(&drifted).unwrap();
        let record = plan_target(&drifted).unwrap();
        assert_eq!(inspection.classification(), "partial");
        assert_eq!(record.mutation_count(), 1);
        let mutation = &record.plan.mutations[0];
        assert_eq!(mutation.path, "scripts/check");
        assert_eq!(mutation.rollback.unix_mode, Some(drifted_mode));
        assert_eq!(mutation.replacement_unix_mode, 0o755);

        execute(&fixture, fixture.plan(&drifted)).unwrap();
        assert_eq!(fs::metadata(&target).unwrap().mode() & 0o7777, 0o755);
        assert!(verify_target(&fixture.context()).unwrap().idempotent());
    }
}
