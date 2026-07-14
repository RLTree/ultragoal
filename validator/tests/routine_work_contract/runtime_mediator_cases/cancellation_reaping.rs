use super::*;

#[test]
#[cfg(target_os = "macos")]
pub(crate) fn cancellation_kills_the_process_group_and_emits_no_reuse() {
    let _serial = mediator_lock();
    let cancellation_fixture = fixture("mediator-cancel", true);
    let prepared = prepared_with(
        &cancellation_fixture,
        |node_id| adversary_script(node_id, "loop"),
        10_000,
        1024 * 1024,
    );
    let grant = issue_grant(&prepared, "cancel-session", None);
    let cancellation = RoutineCancellation::new();
    let trigger = cancellation.clone();
    let syntax = cancellation_fixture
        .repo
        .root()
        .join("target/routine/syntax");
    let verifier_syntax = syntax.clone();
    let canceller = std::thread::spawn(move || {
        let process = wait_for_live_reported_process(&verifier_syntax);
        trigger.cancel();
        process
    });
    let result = mediate(
        &cancellation_fixture,
        prepared,
        Some(grant),
        cancellation,
        Vec::new(),
    );
    let process = canceller.join().unwrap();
    assert_eq!(result.status(), RoutineMediatorStatus::Cancelled);
    assert!(result.reuse_artifacts().is_empty());
    assert!(result.recovery_marker().is_some());
    assert_process_absent(process.pid);
    assert_process_group_absent(process.pgid);
    assert_output_stopped(&process.activity, "cancellation");

    let partial_fixture = fixture("mediator-cancel-after-complete-node", true);
    let partial_prepared = prepared_with(
        &partial_fixture,
        |node_id| {
            if node_id == "syntax" {
                command_script(node_id)
            } else {
                adversary_script(node_id, "loop")
            }
        },
        10_000,
        1024 * 1024,
    );
    let partial_grant = issue_grant(&partial_prepared, "cancel-after-complete", None);
    let partial_cancellation = RoutineCancellation::new();
    let trigger = partial_cancellation.clone();
    let compile_scope = partial_fixture.repo.root().join("target/routine/compile");
    let canceller = std::thread::spawn(move || {
        let process = wait_for_live_reported_process(&compile_scope);
        trigger.cancel();
        process
    });
    let partial = mediate(
        &partial_fixture,
        partial_prepared,
        Some(partial_grant),
        partial_cancellation,
        Vec::new(),
    );
    let partial_process = canceller.join().unwrap();
    assert_eq!(partial.status(), RoutineMediatorStatus::Cancelled);
    assert_eq!(
        partial.nodes()[0].disposition(),
        RoutineNodeDisposition::Executed
    );
    assert_eq!(
        partial.nodes()[1].disposition(),
        RoutineNodeDisposition::Cancelled
    );
    assert!(partial.reuse_artifacts().is_empty());
    assert!(
        partial.recovery_marker().is_some(),
        "started completed work still requires explicit whole-batch recovery"
    );
    assert_process_absent(partial_process.pid);
    assert_process_group_absent(partial_process.pgid);
    assert_output_stopped(
        &partial_process.activity,
        "cancellation-after-complete-node",
    );
}

#[test]
#[cfg(target_os = "macos")]
pub(crate) fn timeout_requires_exact_recovery_authority_and_leaves_no_child() {
    let _serial = mediator_lock();
    let fixture = fixture("mediator-timeout-recovery", true);
    let slow = |node_id: &str| adversary_script(node_id, "loop");
    let syntax = fixture.repo.root().join("target/routine/syntax");
    let verifier_syntax = syntax.clone();
    let verifier = std::thread::spawn(move || wait_for_live_reported_process(&verifier_syntax));
    let first_prepared = prepared_with(&fixture, &slow, 500, 1024 * 1024);
    let first_grant = issue_grant(&first_prepared, "timeout-session-1", None);
    let first = mediate(
        &fixture,
        first_prepared,
        Some(first_grant),
        RoutineCancellation::new(),
        Vec::new(),
    );
    let first_process = verifier.join().unwrap();
    assert_eq!(first.status(), RoutineMediatorStatus::IncompleteExecution);
    assert_eq!(first.nodes()[0].failure_code(), Some("MEDIATOR-TIMEOUT"));
    assert!(first.reuse_artifacts().is_empty());
    let marker = first.recovery_marker().unwrap().to_owned();
    assert_process_absent(first_process.pid);
    assert_process_group_absent(first_process.pgid);
    assert_output_stopped(&first_process.activity, "timeout");

    let refused_prepared = prepared_with(&fixture, &slow, 500, 1024 * 1024);
    let refused_grant = issue_grant(&refused_prepared, "timeout-session-2", None);
    let before_refusal = test_spawn_count();
    let refusal = mediate_prepared_routine_execution(
        &fixture.context,
        &fixture.plan,
        refused_prepared,
        Some(refused_grant),
        RoutineCancellation::new(),
        RoutineReuseInput::default(),
    )
    .expect_err("ambiguous started work must require recovery authority");
    assert_eq!(refusal.cause(), "mediator-recovery-authority-required");
    assert_eq!(test_spawn_count(), before_refusal);

    let recovery_syntax = syntax.clone();
    let recovery_verifier =
        std::thread::spawn(move || wait_for_live_reported_process(&recovery_syntax));
    let recovery_prepared = prepared_with(&fixture, &slow, 500, 1024 * 1024);
    let recovery_grant = issue_grant(&recovery_prepared, "timeout-session-3", Some(marker));
    let recovered_attempt = mediate(
        &fixture,
        recovery_prepared,
        Some(recovery_grant),
        RoutineCancellation::new(),
        Vec::new(),
    );
    let recovery_process = recovery_verifier.join().unwrap();
    assert_eq!(
        recovered_attempt.status(),
        RoutineMediatorStatus::IncompleteExecution
    );
    assert!(recovered_attempt.recovery_marker().is_some());
    assert_process_absent(recovery_process.pid);
    assert_process_group_absent(recovery_process.pgid);
    assert_output_stopped(&recovery_process.activity, "timeout-recovery");
}
