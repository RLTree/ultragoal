use super::*;

#[test]
#[cfg(target_os = "macos")]
pub(crate) fn post_spawn_context_mutation_requires_exact_recovery_before_retry() {
    let _serial = mediator_lock();
    let fixture = fixture("mediator-post-spawn-context-recovery", true);
    let first_prepared = prepared(&fixture);
    let first_grant = issue_grant(&first_prepared, "post-spawn-context-1", None);
    let marker = first_grant.test_recovery_marker();
    let root = fixture.repo.root().to_owned();
    set_test_mediator_post_spawn_hook(move || {
        fs::write(root.join("src/lib.rs"), b"pub fn value() -> u8 { 60 }\n").unwrap();
    });
    let before_spawns = test_spawn_count();
    let refused = mediate_prepared_routine_execution(
        &fixture.context,
        &fixture.plan,
        first_prepared,
        Some(first_grant),
        RoutineCancellation::new(),
        RoutineReuseInput::default(),
    )
    .expect_err("post-spawn context mutation must refuse final reconciliation");
    assert_eq!(refused.cause(), "mediator-result-context-stale");
    assert_eq!(test_spawn_count(), before_spawns + 1);
    assert_eq!(
        fs::read(fixture.repo.root().join("target/routine/syntax/result.txt")).unwrap(),
        b"syntax"
    );
    assert!(
        !fixture
            .repo
            .root()
            .join("target/routine/compile/result.txt")
            .exists()
    );

    fixture
        .repo
        .write("src/lib.rs", b"pub fn value() -> u8 { 59 }\n");
    fixture.context.revalidate().unwrap();

    let retry_prepared = prepared(&fixture);
    let retry_grant = issue_grant(&retry_prepared, "post-spawn-context-2", None);
    let retry = mediate_prepared_routine_execution(
        &fixture.context,
        &fixture.plan,
        retry_prepared,
        Some(retry_grant),
        RoutineCancellation::new(),
        RoutineReuseInput::default(),
    )
    .expect_err("fresh non-recovery authority must not silently retry started work");
    assert_eq!(retry.cause(), "mediator-recovery-authority-required");
    assert_eq!(test_spawn_count(), before_spawns + 1);

    let wrong_prepared = prepared(&fixture);
    let wrong_grant = issue_grant(
        &wrong_prepared,
        "post-spawn-context-3",
        Some(format!("sha256:{}", "f".repeat(64))),
    );
    let wrong = mediate_prepared_routine_execution(
        &fixture.context,
        &fixture.plan,
        wrong_prepared,
        Some(wrong_grant),
        RoutineCancellation::new(),
        RoutineReuseInput::default(),
    )
    .expect_err("substituted recovery marker must not clear ambiguity");
    assert_eq!(wrong.cause(), "mediator-recovery-authority-required");
    assert_eq!(test_spawn_count(), before_spawns + 1);

    let recovery_prepared = prepared(&fixture);
    let recovery_grant = issue_grant(&recovery_prepared, "post-spawn-context-4", Some(marker));
    let recovered = mediate(
        &fixture,
        recovery_prepared,
        Some(recovery_grant),
        RoutineCancellation::new(),
        Vec::new(),
    );
    assert_eq!(recovered.status(), RoutineMediatorStatus::CompleteExecution);
    assert!(recovered.recovery_marker().is_none());
    assert!(recovered.nodes().iter().all(|node| {
        node.disposition() == RoutineNodeDisposition::Executed
            && node.result_artifact_sha256().is_some()
    }));
    assert_eq!(recovered.reuse_artifacts().len(), 3);
    assert_eq!(test_spawn_count(), before_spawns + 4);
}

#[test]
#[cfg(target_os = "macos")]
pub(crate) fn post_spawn_finish_failure_keeps_the_exact_recovery_barrier() {
    let _serial = mediator_lock();
    let fixture = fixture("mediator-post-spawn-finish-recovery", true);
    let first_prepared = prepared(&fixture);
    let first_grant = issue_grant(&first_prepared, "post-spawn-finish-1", None);
    let marker = first_grant.test_recovery_marker();
    set_test_mediator_finish_failure();
    let before_spawns = test_spawn_count();
    let refused = mediate_prepared_routine_execution(
        &fixture.context,
        &fixture.plan,
        first_prepared,
        Some(first_grant),
        RoutineCancellation::new(),
        RoutineReuseInput::default(),
    )
    .expect_err("finish failure after execution must not expose success");
    assert_eq!(refused.cause(), "adapter-mediation-transition-incomplete");
    assert_eq!(test_spawn_count(), before_spawns + 3);

    let retry_prepared = prepared(&fixture);
    let retry_grant = issue_grant(&retry_prepared, "post-spawn-finish-2", None);
    let retry = mediate_prepared_routine_execution(
        &fixture.context,
        &fixture.plan,
        retry_prepared,
        Some(retry_grant),
        RoutineCancellation::new(),
        RoutineReuseInput::default(),
    )
    .expect_err("finish failure must leave the protocol recovery-blocked");
    assert_eq!(retry.cause(), "mediator-recovery-authority-required");
    assert_eq!(test_spawn_count(), before_spawns + 3);

    let recovery_prepared = prepared(&fixture);
    let recovery_grant = issue_grant(&recovery_prepared, "post-spawn-finish-3", Some(marker));
    let recovered = mediate(
        &fixture,
        recovery_prepared,
        Some(recovery_grant),
        RoutineCancellation::new(),
        Vec::new(),
    );
    assert_eq!(recovered.status(), RoutineMediatorStatus::CompleteExecution);
    assert!(recovered.recovery_marker().is_none());
    assert_eq!(test_spawn_count(), before_spawns + 6);
}
