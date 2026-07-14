use super::*;

#[test]
#[cfg(unix)]
pub(crate) fn read_source_binding_refuses_symlinks_hardlinks_special_files_and_capture_races() {
    let _serial = mediator_lock();

    let symlink_fixture = fixture("mediator-read-source-symlink", true);
    symlink_fixture
        .repo
        .write("target/routine/real.sh", b"exit 0\n");
    std::os::unix::fs::symlink(
        "real.sh",
        symlink_fixture.repo.root().join("target/routine/source.sh"),
    )
    .unwrap();
    let symlink = bind_routine_invocation_with_read_sources(
        &symlink_fixture.context,
        &symlink_fixture.plan,
        "syntax",
        vec!["target/routine/source.sh".to_owned()],
        vec![path("target/routine/source.sh")],
        1_000,
        1_024,
        vec![path("target/routine/syntax")],
    )
    .unwrap_err();
    assert_eq!(symlink.cause(), "mediator-read-source-object-unsafe");

    let hardlink_fixture = fixture("mediator-read-source-hardlink", true);
    hardlink_fixture
        .repo
        .write("target/routine/backing.sh", b"exit 0\n");
    fs::hard_link(
        hardlink_fixture
            .repo
            .root()
            .join("target/routine/backing.sh"),
        hardlink_fixture
            .repo
            .root()
            .join("target/routine/source.sh"),
    )
    .unwrap();
    let hardlink = bind_routine_invocation_with_read_sources(
        &hardlink_fixture.context,
        &hardlink_fixture.plan,
        "syntax",
        vec!["target/routine/source.sh".to_owned()],
        vec![path("target/routine/source.sh")],
        1_000,
        1_024,
        vec![path("target/routine/syntax")],
    )
    .unwrap_err();
    assert_eq!(hardlink.cause(), "mediator-read-source-object-unsafe");

    let fifo_fixture = fixture("mediator-read-source-fifo", true);
    let fifo = fifo_fixture.repo.root().join("target/routine/source.fifo");
    let fifo_bytes = std::ffi::CString::new(fifo.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(fifo_bytes.as_ptr(), 0o600) }, 0);
    let special = bind_routine_invocation_with_read_sources(
        &fifo_fixture.context,
        &fifo_fixture.plan,
        "syntax",
        vec!["target/routine/source.fifo".to_owned()],
        vec![path("target/routine/source.fifo")],
        1_000,
        1_024,
        vec![path("target/routine/syntax")],
    )
    .unwrap_err();
    assert_eq!(special.cause(), "mediator-read-source-object-unsafe");

    let race_fixture = fixture("mediator-read-source-capture-race", true);
    let source = race_fixture.repo.root().join("target/routine/source.sh");
    let moved = race_fixture
        .repo
        .root()
        .join("target/routine/source-moved.sh");
    fs::write(&source, b"exit 0\n").unwrap();
    let hook_source = source.clone();
    let hook_moved = moved.clone();
    set_test_read_source_capture_hook(move || {
        fs::rename(&hook_source, &hook_moved).unwrap();
        std::os::unix::fs::symlink("source-moved.sh", &hook_source).unwrap();
    });
    let raced = bind_routine_invocation_with_read_sources(
        &race_fixture.context,
        &race_fixture.plan,
        "syntax",
        vec!["target/routine/source.sh".to_owned()],
        vec![path("target/routine/source.sh")],
        1_000,
        1_024,
        vec![path("target/routine/syntax")],
    )
    .unwrap_err();
    assert!(matches!(
        raced.cause(),
        "mediator-read-source-mutated-during-capture"
            | "mediator-read-source-replaced-during-capture"
    ));
    fs::remove_file(&source).unwrap();
    fs::rename(&moved, &source).unwrap();
}

#[test]
#[cfg(target_os = "macos")]
pub(crate) fn read_source_aba_before_spawn_is_refused_without_effect() {
    let _serial = mediator_lock();
    let fixture = fixture("mediator-read-source-aba", true);
    let relative_source = "target/routine/source.sh";
    let source = fixture.repo.root().join(relative_source);
    let moved = fixture
        .repo
        .root()
        .join("target/routine/source-original.sh");
    fs::write(&source, command_file_script()).unwrap();
    let prepared = prepared_with_read_source(
        &fixture,
        |_| vec![relative_source.to_owned()],
        relative_source,
        10_000,
        1024 * 1024,
    );
    let grant = issue_grant(&prepared, "read-source-aba", None);
    let hook_source = source.clone();
    let hook_moved = moved.clone();
    set_test_mediator_pre_spawn_hook(move || {
        fs::rename(&hook_source, &hook_moved).unwrap();
        fs::write(
            &hook_source,
            b"printf false-pass > target/routine/syntax/false-pass\n",
        )
        .unwrap();
        fs::remove_file(&hook_source).unwrap();
        fs::rename(&hook_moved, &hook_source).unwrap();
    });
    let before_spawns = test_spawn_count();
    let error = mediate_prepared_routine_execution(
        &fixture.context,
        &fixture.plan,
        prepared,
        Some(grant),
        RoutineCancellation::new(),
        RoutineReuseInput::default(),
    )
    .expect_err("read-source mutate/restore ambiguity must fail the second preflight");
    assert_eq!(error.cause(), "mediator-read-source-binding-stale");
    assert_eq!(test_spawn_count(), before_spawns);
    assert!(
        !fixture
            .repo
            .root()
            .join("target/routine/syntax/false-pass")
            .exists()
    );
}

#[test]
#[cfg(target_os = "macos")]
pub(crate) fn successful_exit_cannot_hide_post_spawn_read_source_mutation() {
    let _serial = mediator_lock();
    let fixture = fixture("mediator-read-source-false-pass", true);
    let relative_source = "target/routine/source.sh";
    let source = fixture.repo.root().join(relative_source);
    let original = command_file_script();
    fs::write(&source, &original).unwrap();
    let prepared = prepared_with_read_source(
        &fixture,
        |_| vec![relative_source.to_owned()],
        relative_source,
        10_000,
        1024 * 1024,
    );
    let grant = issue_grant(&prepared, "read-source-false-pass", None);
    let hook_source = source.clone();
    set_test_mediator_post_spawn_hook(move || {
        fs::write(&hook_source, b"exit 71\n").unwrap();
        fs::write(&hook_source, original).unwrap();
    });
    let before_spawns = test_spawn_count();
    let result = mediate(
        &fixture,
        prepared,
        Some(grant),
        RoutineCancellation::new(),
        Vec::new(),
    );
    assert_eq!(result.status(), RoutineMediatorStatus::IncompleteExecution);
    assert_eq!(
        result.nodes()[0].failure_code(),
        Some("MEDIATOR-READ-SOURCE-CHANGED")
    );
    assert!(result.reuse_artifacts().is_empty());
    assert!(result.recovery_marker().is_some());
    assert_eq!(test_spawn_count(), before_spawns + 1);
    assert_eq!(
        fs::read(fixture.repo.root().join("target/routine/syntax/result.txt")).unwrap(),
        b"syntax",
        "the canonical passing child reached its authorized effect before causal validation"
    );
    assert!(
        result
            .nodes()
            .iter()
            .all(|node| node.result_artifact_sha256().is_none())
    );
}
