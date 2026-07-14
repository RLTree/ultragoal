use super::*;

#[test]
#[cfg(target_os = "macos")]
pub(crate) fn post_spawn_setup_failures_cleanup_every_resource_and_require_exact_recovery() {
    let _serial = mediator_lock();
    let cases = [
        (
            TestProcessSetupFailure::ProcessGroup,
            "process-group",
            "MEDIATOR-PROCESS-GROUP-SETUP-INJECTED",
        ),
        (
            TestProcessSetupFailure::StdoutNonblocking,
            "stdout-nonblocking",
            "MEDIATOR-STDOUT-NONBLOCKING-INJECTED",
        ),
        (
            TestProcessSetupFailure::StderrNonblocking,
            "stderr-nonblocking",
            "MEDIATOR-STDERR-NONBLOCKING-INJECTED",
        ),
        (
            TestProcessSetupFailure::StdoutReaderStart,
            "stdout-reader-start",
            "MEDIATOR-STDOUT-READER-START-INJECTED",
        ),
        (
            TestProcessSetupFailure::StderrReaderStart,
            "stderr-reader-start",
            "MEDIATOR-STDERR-READER-START-INJECTED",
        ),
    ];

    for (point, label, expected_failure) in cases {
        let fixture = fixture(&format!("mediator-setup-{label}"), true);
        let first_prepared = prepared_with(&fixture, setup_failure_script, 10_000, 1024 * 1024);
        let first_grant = issue_grant(&first_prepared, &format!("setup-{label}-1"), None);
        let marker = first_grant.test_recovery_marker();
        let syntax = fixture.repo.root().join("target/routine/syntax");
        let hook_syntax = syntax.clone();
        set_test_process_setup_failure(point, move || {
            let process = wait_for_live_reported_process(&hook_syntax);
            assert_eq!(
                unsafe { libc::getpgid(process.pid) },
                process.pgid,
                "case={label} runner left the reported process group before injection"
            );
        });

        let before_spawns = test_spawn_count();
        let first = mediate(
            &fixture,
            first_prepared,
            Some(first_grant),
            RoutineCancellation::new(),
            Vec::new(),
        );
        assert_eq!(
            first.status(),
            RoutineMediatorStatus::IncompleteExecution,
            "case={label} nodes={:?}",
            first.nodes()
        );
        assert_eq!(first.nodes()[0].failure_code(), Some(expected_failure));
        assert!(
            first
                .nodes()
                .iter()
                .all(|node| node.result_artifact_sha256().is_none()),
            "case={label} exposed a success artifact"
        );
        assert!(first.reuse_artifacts().is_empty());
        assert_eq!(first.recovery_marker(), Some(marker.as_str()));
        assert_eq!(test_spawn_count(), before_spawns + 1);

        let process = read_reported_process(&syntax).unwrap();
        assert_process_absent(process.pid);
        assert_process_group_absent(process.pgid);
        assert_output_stopped(&process.activity, label);

        let retry_prepared = prepared_with(&fixture, setup_failure_script, 10_000, 1024 * 1024);
        let retry_grant = issue_grant(&retry_prepared, &format!("setup-{label}-2"), None);
        let retry = mediate_prepared_routine_execution(
            &fixture.context,
            &fixture.plan,
            retry_prepared,
            Some(retry_grant),
            RoutineCancellation::new(),
            RoutineReuseInput::default(),
        )
        .expect_err("fresh non-recovery authority must not retry setup-ambiguous work");
        assert_eq!(retry.cause(), "mediator-recovery-authority-required");
        assert_eq!(test_spawn_count(), before_spawns + 1);

        let wrong_prepared = prepared_with(&fixture, setup_failure_script, 10_000, 1024 * 1024);
        let wrong_grant = issue_grant(
            &wrong_prepared,
            &format!("setup-{label}-3"),
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
        .expect_err("substituted setup recovery marker must not proceed");
        assert_eq!(wrong.cause(), "mediator-recovery-authority-required");
        assert_eq!(test_spawn_count(), before_spawns + 1);

        let recovery_prepared = prepared_with(&fixture, setup_failure_script, 10_000, 1024 * 1024);
        let recovery_grant = issue_grant(
            &recovery_prepared,
            &format!("setup-{label}-4"),
            Some(marker),
        );
        let recovered = mediate(
            &fixture,
            recovery_prepared,
            Some(recovery_grant),
            RoutineCancellation::new(),
            Vec::new(),
        );
        assert_eq!(
            recovered.status(),
            RoutineMediatorStatus::CompleteExecution,
            "case={label} nodes={:?}",
            recovered.nodes()
        );
        assert!(recovered.recovery_marker().is_none());
        assert!(recovered.nodes().iter().all(|node| {
            node.disposition() == RoutineNodeDisposition::Executed
                && node.result_artifact_sha256().is_some()
        }));
        assert_eq!(recovered.reuse_artifacts().len(), 3);
        assert_eq!(test_spawn_count(), before_spawns + 4);
    }
}

#[test]
#[cfg(target_os = "macos")]
pub(crate) fn missing_forged_and_replayed_root_authority_fail_closed() {
    let _serial = mediator_lock();
    let fixture = fixture("mediator-authority", true);
    let before_spawns = test_spawn_count();

    let missing = mediate_prepared_routine_execution(
        &fixture.context,
        &fixture.plan,
        prepared(&fixture),
        None,
        RoutineCancellation::new(),
        RoutineReuseInput::default(),
    )
    .unwrap_err();
    assert_eq!(missing.cause(), "mediator-root-grant-missing");

    let forged_prepared = prepared(&fixture);
    let forged = issue_grant(&forged_prepared, "forged-session", None)
        .test_with_seal("sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff");
    let error = mediate_prepared_routine_execution(
        &fixture.context,
        &fixture.plan,
        forged_prepared,
        Some(forged),
        RoutineCancellation::new(),
        RoutineReuseInput::default(),
    )
    .unwrap_err();
    assert_eq!(error.cause(), "mediator-root-grant-binding-invalid");

    for (label, grant) in [
        {
            let value = prepared(&fixture);
            let grant = issue_grant(&value, "cross-session", None)
                .test_with_session_id("substituted-session");
            ("session", (value, grant))
        },
        {
            let value = prepared(&fixture);
            let grant = issue_grant(&value, "cross-context", None)
                .test_with_context_id(format!("sha256:{}", "a".repeat(64)));
            ("context", (value, grant))
        },
        {
            let value = prepared(&fixture);
            let grant = issue_grant(&value, "cross-plan", None)
                .test_with_plan_id(format!("sha256:{}", "b".repeat(64)));
            ("plan", (value, grant))
        },
        {
            let value = prepared(&fixture);
            let grant =
                issue_grant(&value, "expanded-scope", None).test_with_scopes(vec![path("target")]);
            ("scope", (value, grant))
        },
    ] {
        let (value, grant) = grant;
        let error = mediate_prepared_routine_execution(
            &fixture.context,
            &fixture.plan,
            value,
            Some(grant),
            RoutineCancellation::new(),
            RoutineReuseInput::default(),
        )
        .expect_err("substituted root grant must fail");
        assert_eq!(
            error.cause(),
            "mediator-root-grant-binding-invalid",
            "variant={label}"
        );
    }

    let replay_prepared = prepared(&fixture);
    let duplicate_request = effect_request(&replay_prepared).test_duplicate();
    let replay_grant = issue_grant(&replay_prepared, "replay-session", None);
    let duplicate_grant = replay_grant.test_duplicate();
    let completed = mediate(
        &fixture,
        replay_prepared,
        Some(replay_grant),
        RoutineCancellation::new(),
        Vec::new(),
    );
    assert_eq!(completed.status(), RoutineMediatorStatus::CompleteExecution);
    let replay = mediate_prepared_routine_execution(
        &fixture.context,
        &fixture.plan,
        PreparedRoutineExecution::Effect(duplicate_request),
        Some(duplicate_grant),
        RoutineCancellation::new(),
        RoutineReuseInput::default(),
    )
    .unwrap_err();
    assert_eq!(replay.cause(), "mediator-root-grant-replayed");
    assert_eq!(test_spawn_count(), before_spawns + 3);
}
