use super::*;

#[test]
pub(crate) fn unsafe_output_objects_are_rejected_before_authority_is_consumed() {
    let _serial = mediator_lock();
    let before_spawns = test_spawn_count();

    let symlink_fixture = fixture("mediator-output-symlink", true);
    let symlink_prepared = prepared(&symlink_fixture);
    let grant = issue_grant(&symlink_prepared, "output-symlink-session", None);
    #[cfg(unix)]
    std::os::unix::fs::symlink(
        "../compile",
        symlink_fixture
            .repo
            .root()
            .join("target/routine/syntax/escape"),
    )
    .unwrap();
    let symlink = mediate_prepared_routine_execution(
        &symlink_fixture.context,
        &symlink_fixture.plan,
        symlink_prepared,
        Some(grant),
        RoutineCancellation::new(),
        RoutineReuseInput::default(),
    )
    .expect_err("symlink output must be rejected");
    assert_eq!(symlink.cause(), "mediator-output-object-unsafe");

    let hardlink_fixture = fixture("mediator-output-hardlink", true);
    hardlink_fixture
        .repo
        .write("target/routine/backing", b"same inode\n");
    fs::hard_link(
        hardlink_fixture.repo.root().join("target/routine/backing"),
        hardlink_fixture
            .repo
            .root()
            .join("target/routine/syntax/hardlink"),
    )
    .unwrap();
    let hardlink_prepared = prepared(&hardlink_fixture);
    let grant = issue_grant(&hardlink_prepared, "output-hardlink-session", None);
    let hardlink = mediate_prepared_routine_execution(
        &hardlink_fixture.context,
        &hardlink_fixture.plan,
        hardlink_prepared,
        Some(grant),
        RoutineCancellation::new(),
        RoutineReuseInput::default(),
    )
    .expect_err("hardlinked output must be rejected");
    assert_eq!(hardlink.cause(), "mediator-output-hardlink-refused");

    #[cfg(unix)]
    {
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt;
        let fifo_fixture = fixture("mediator-output-fifo", true);
        let fifo = fifo_fixture.repo.root().join("target/routine/syntax/fifo");
        let fifo_bytes = CString::new(fifo.as_os_str().as_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(fifo_bytes.as_ptr(), 0o600) }, 0);
        let prepared = prepared(&fifo_fixture);
        let grant = issue_grant(&prepared, "output-fifo-session", None);
        let special = mediate_prepared_routine_execution(
            &fifo_fixture.context,
            &fifo_fixture.plan,
            prepared,
            Some(grant),
            RoutineCancellation::new(),
            RoutineReuseInput::default(),
        )
        .expect_err("special output object must be rejected");
        assert_eq!(special.cause(), "mediator-output-object-unsafe");
    }
    assert_eq!(test_spawn_count(), before_spawns);
}

#[test]
pub(crate) fn descriptor_walk_refuses_a_nested_directory_swap_during_capture() {
    let _serial = mediator_lock();
    let fixture = fixture("mediator-nested-capture-race", true);
    let nested = fixture.repo.root().join("target/routine/syntax/nested");
    fs::create_dir_all(&nested).unwrap();
    fixture
        .repo
        .write("target/routine/syntax/nested/value", b"captured\n");
    let moved = fixture
        .repo
        .root()
        .join("target/routine/syntax/nested-moved");
    let hook_nested = nested.clone();
    set_test_output_capture_hook(move || {
        fs::rename(&hook_nested, moved).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink("../compile", hook_nested).unwrap();
    });
    let prepared = prepared(&fixture);
    let grant = issue_grant(&prepared, "nested-capture-race", None);
    let before_spawns = test_spawn_count();
    let error = mediate_prepared_routine_execution(
        &fixture.context,
        &fixture.plan,
        prepared,
        Some(grant),
        RoutineCancellation::new(),
        RoutineReuseInput::default(),
    )
    .expect_err("descriptor-relative capture must reject nested path replacement");
    assert_eq!(error.cause(), "mediator-output-object-unsafe");
    assert_eq!(test_spawn_count(), before_spawns);
}

#[test]
#[cfg(target_os = "macos")]
pub(crate) fn cleared_environment_and_correlated_child_report_are_required() {
    let _serial = mediator_lock();
    let environment_fixture = fixture("mediator-environment-report", true);
    let guarded = |node_id: &str| {
        format!(
            "test \"$LANG\" = C && test \"$LC_ALL\" = C && test \"$PATH\" = /bin && test -z \"${{SSH_AUTH_SOCK+x}}\" || exit 70; printf '%s' '{node_id}' > 'target/routine/{node_id}/result.txt'; {}",
            report_script()
        )
    };
    let prepared = prepared_with(&environment_fixture, guarded, 10_000, 1024 * 1024);
    let grant = issue_grant(&prepared, "environment-session", None);
    let result = mediate(
        &environment_fixture,
        prepared,
        Some(grant),
        RoutineCancellation::new(),
        Vec::new(),
    );
    assert_eq!(result.status(), RoutineMediatorStatus::CompleteExecution);

    let missing_report_fixture = fixture("mediator-missing-report", true);
    let prepared = prepared_with(
        &missing_report_fixture,
        |_| "true".to_owned(),
        10_000,
        1024 * 1024,
    );
    let grant = issue_grant(&prepared, "missing-report-session", None);
    let result = mediate(
        &missing_report_fixture,
        prepared,
        Some(grant),
        RoutineCancellation::new(),
        Vec::new(),
    );
    assert_eq!(result.status(), RoutineMediatorStatus::IncompleteExecution);
    assert_eq!(result.reuse_artifacts().len(), 0);
    assert_eq!(
        result.nodes()[0].failure_code(),
        Some("MEDIATOR-COMMAND-REPORT-INVALID")
    );
    assert!(result.recovery_marker().is_some());
}

#[test]
#[cfg(target_os = "macos")]
pub(crate) fn sandbox_denies_undeclared_writes_and_network_connections() {
    let _serial = mediator_lock();
    let write_fixture = fixture("mediator-undeclared-write", true);
    let forbidden = write_fixture.repo.root().join("forbidden.txt");
    let prepared = prepared_with(
        &write_fixture,
        |_| "set -e; printf denied > forbidden.txt".to_owned(),
        10_000,
        1024 * 1024,
    );
    let grant = issue_grant(&prepared, "undeclared-write-session", None);
    let result = mediate(
        &write_fixture,
        prepared,
        Some(grant),
        RoutineCancellation::new(),
        Vec::new(),
    );
    assert_eq!(result.status(), RoutineMediatorStatus::IncompleteExecution);
    assert!(!forbidden.exists());
    assert!(result.reuse_artifacts().is_empty());

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let port = listener.local_addr().unwrap().port();
    let network_fixture = fixture("mediator-network-denied", true);
    let prepared = prepared_with(
        &network_fixture,
        |_| format!("exec /usr/bin/nc -w 1 127.0.0.1 {port}"),
        10_000,
        1024 * 1024,
    );
    let grant = issue_grant(&prepared, "network-denied-session", None);
    let result = mediate(
        &network_fixture,
        prepared,
        Some(grant),
        RoutineCancellation::new(),
        Vec::new(),
    );
    assert_eq!(result.status(), RoutineMediatorStatus::IncompleteExecution);
    assert!(result.reuse_artifacts().is_empty());
    assert!(
        listener.accept().is_err(),
        "sandboxed child reached listener"
    );
}
