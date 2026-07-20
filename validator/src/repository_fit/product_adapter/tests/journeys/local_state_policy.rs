use super::*;
use std::process::Stdio;

#[test]
pub(crate) fn local_state_only_mutation_is_counted_at_each_preparation_surface() {
    let fixture = Fixture::new("local-state-only-count");
    fixture.install_all_direct();
    fs::remove_file(fixture.root.join(".gitignore")).unwrap();
    let context = fixture.context();
    let record = plan_target(&context).unwrap();
    assert_eq!(record.mutation_count(), 1);
    assert!(record.plan.local_state.mutation_required);
    let prepared = fixture.plan(&context);
    assert_eq!(prepared.projection().mutation_count, 1);
    assert_eq!(prepared.request().all_mutations().len(), 1);
    execute(&fixture, prepared).unwrap();
    assert_eq!(
        fs::read(fixture.root.join(".gitignore")).unwrap(),
        b"validation_artifacts/\n"
    );
}

#[test]
pub(crate) fn absent_local_state_is_created_with_canonical_rule() {
    let fixture = Fixture::new("local-state-absent");
    let context = fixture.context();
    let inspection = assert_zero_write(&fixture, || inspect_target(&context).unwrap());
    assert_eq!(inspection.local_state.disposition, "missing");
    let prepared = fixture.plan(&context);
    execute(&fixture, prepared).unwrap();
    assert_eq!(
        fs::read(fixture.root.join(".gitignore")).unwrap(),
        b"validation_artifacts/\n"
    );
    assert_eq!(
        fs::metadata(fixture.root.join(".gitignore"))
            .unwrap()
            .mode()
            & 0o7777,
        0o644
    );
}

#[test]
pub(crate) fn local_state_policy_preserves_user_bytes_newline_shape_and_mode() {
    let fixture = Fixture::new("local-state-policy");
    fixture.write(
        ".gitignore",
        b"# repository-owned notes\r\n!validation_artifacts/\r\nprivate/",
    );
    fs::set_permissions(
        fixture.root.join(".gitignore"),
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    fixture.write("unrelated.txt", b"keep me\n");
    let before_status = git_status(&fixture.root);
    let context = fixture.context();
    let inspection = assert_zero_write(&fixture, || inspect_target(&context).unwrap());
    assert_eq!(inspection.local_state.disposition, "needs_update");
    let prepared = fixture.plan(&context);
    execute(&fixture, prepared).unwrap();

    assert_eq!(
        fs::read(fixture.root.join(".gitignore")).unwrap(),
        b"# repository-owned notes\r\n!validation_artifacts/\r\nprivate/\r\nvalidation_artifacts/"
    );
    assert_eq!(
        fs::metadata(fixture.root.join(".gitignore"))
            .unwrap()
            .mode()
            & 0o7777,
        0o600
    );
    assert_eq!(
        fs::read(fixture.root.join("unrelated.txt")).unwrap(),
        b"keep me\n"
    );
    assert_ne!(git_status(&fixture.root), before_status);

    let child = fixture
        .root
        .join("validation_artifacts/observability/spool/successor-events.jsonl");
    fixture.write("validation_artifacts/observability/spool/child.txt", b"x");
    git(
        &fixture.root,
        &["check-ignore", "--quiet", child.to_str().unwrap()],
    );
    let tracked = std::process::Command::new("git")
        .args([
            "ls-files",
            "--error-unmatch",
            "--",
            "validation_artifacts/observability/spool/child.txt",
        ])
        .current_dir(&fixture.root)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();
    assert_eq!(tracked.code(), Some(1));
}

#[test]
pub(crate) fn already_ignored_local_state_is_a_repeat_noop() {
    let fixture = Fixture::new("local-state-repeat");
    fixture.install_all_direct();
    fixture.write(".gitignore", b"# keep\nvalidation_artifacts/\n");
    let context = fixture.context();
    let record = plan_target(&context).unwrap();
    assert!(!record.plan.local_state.mutation_required);
    let first = fixture.plan(&context);
    let before = snapshot(&fixture.root);
    execute(&fixture, first).unwrap();
    assert_eq!(snapshot(&fixture.root), before);
    assert!(verify_target(&fixture.context()).unwrap().idempotent());
}
