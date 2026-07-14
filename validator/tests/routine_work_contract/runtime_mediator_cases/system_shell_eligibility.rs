use super::*;

#[test]
#[cfg(target_os = "macos")]
pub(crate) fn root_owned_system_shell_remains_eligible_for_non_root_execution() {
    let _serial = mediator_lock();
    let fixture = fixture("mediator-system-shell", true);
    let selected = fixture
        .context
        .capabilities()
        .tool("dash")
        .and_then(|tool| tool.executable.as_deref())
        .unwrap();
    assert_eq!(selected, "/bin/dash");
    let metadata = fs::symlink_metadata(selected).unwrap();
    assert_eq!(metadata.uid(), 0);
    assert_eq!(metadata.mode() & 0o022, 0);

    let prepared = prepared(&fixture);
    let grant = issue_grant(&prepared, "system-shell", None);
    let before_spawns = test_spawn_count();
    if unsafe { libc::geteuid() } == 0 {
        let error = mediate_prepared_routine_execution(
            &fixture.context,
            &fixture.plan,
            prepared,
            Some(grant),
            RoutineCancellation::new(),
            RoutineReuseInput::default(),
        )
        .expect_err("effective root must fail closed for path-based spawn");
        assert_eq!(error.cause(), "mediator-executable-path-mutable");
        assert_eq!(test_spawn_count(), before_spawns);
        return;
    }

    let result = mediate(
        &fixture,
        prepared,
        Some(grant),
        RoutineCancellation::new(),
        Vec::new(),
    );
    assert_eq!(result.status(), RoutineMediatorStatus::CompleteExecution);
    assert_eq!(test_spawn_count(), before_spawns + 3);
}

#[test]
#[cfg(target_os = "macos")]
pub(crate) fn concurrent_duplicate_grant_and_request_have_exactly_one_winner() {
    let _serial = mediator_lock();
    let fixture = fixture("mediator-concurrent-authority", true);
    let first = prepared(&fixture);
    let second_request = effect_request(&first).test_duplicate();
    let first_grant = issue_grant(&first, "concurrent-session", None);
    let second_grant = first_grant.test_duplicate();
    let before_spawns = test_spawn_count();
    let (left, right) = std::thread::scope(|scope| {
        let left = scope.spawn(|| {
            mediate_prepared_routine_execution(
                &fixture.context,
                &fixture.plan,
                first,
                Some(first_grant),
                RoutineCancellation::new(),
                RoutineReuseInput::default(),
            )
        });
        let right = scope.spawn(|| {
            mediate_prepared_routine_execution(
                &fixture.context,
                &fixture.plan,
                PreparedRoutineExecution::Effect(second_request),
                Some(second_grant),
                RoutineCancellation::new(),
                RoutineReuseInput::default(),
            )
        });
        (left.join().unwrap(), right.join().unwrap())
    });
    let outcomes = [left, right];
    assert_eq!(outcomes.iter().filter(|value| value.is_ok()).count(), 1);
    assert_eq!(outcomes.iter().filter(|value| value.is_err()).count(), 1);
    let winner = outcomes
        .iter()
        .find_map(|value| value.as_ref().ok())
        .unwrap();
    assert_eq!(winner.status(), RoutineMediatorStatus::CompleteExecution);
    let loser = outcomes
        .iter()
        .find_map(|value| value.as_ref().err())
        .unwrap();
    assert!(matches!(
        loser.cause(),
        "mediator-root-grant-replayed" | "adapter-mediation-transition-replayed"
    ));
    assert_eq!(test_spawn_count(), before_spawns + 3);
}

#[test]
#[cfg(target_os = "macos")]
pub(crate) fn active_protocol_attempt_blocks_a_distinct_grant_before_second_spawn() {
    let _serial = mediator_lock();
    let fixture = fixture("mediator-concurrent-distinct-authority", true);
    let first_prepared = prepared(&fixture);
    let first_grant = issue_grant(&first_prepared, "concurrent-distinct-1", None);
    let second_prepared = prepared(&fixture);
    let second_grant = issue_grant(&second_prepared, "concurrent-distinct-2", None);
    let (started_tx, started_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    set_test_mediator_post_spawn_hook(move || {
        started_tx.send(()).unwrap();
        release_rx.recv().unwrap();
    });
    let before_spawns = test_spawn_count();
    let (winner, loser) = std::thread::scope(|scope| {
        let winner = scope.spawn(|| {
            mediate_prepared_routine_execution(
                &fixture.context,
                &fixture.plan,
                first_prepared,
                Some(first_grant),
                RoutineCancellation::new(),
                RoutineReuseInput::default(),
            )
        });
        started_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("first child never crossed the successful-spawn boundary");
        let loser = mediate_prepared_routine_execution(
            &fixture.context,
            &fixture.plan,
            second_prepared,
            Some(second_grant),
            RoutineCancellation::new(),
            RoutineReuseInput::default(),
        );
        release_tx.send(()).unwrap();
        (winner.join().unwrap(), loser)
    });
    assert_eq!(
        loser.unwrap_err().cause(),
        "mediator-protocol-attempt-active"
    );
    assert_eq!(
        winner.unwrap().status(),
        RoutineMediatorStatus::CompleteExecution
    );
    assert_eq!(test_spawn_count(), before_spawns + 3);
}

#[test]
pub(crate) fn stale_dirty_bytes_and_output_scope_swap_are_refused_before_spawn() {
    let _serial = mediator_lock();
    let source_fixture = fixture("mediator-stale-source", true);
    let source_prepared = prepared(&source_fixture);
    let grant = issue_grant(&source_prepared, "stale-source-session", None);
    let source = source_fixture.repo.root().join("src/lib.rs");
    set_test_mediator_pre_spawn_hook(move || {
        fs::write(source, b"pub fn value() -> u8 { 60 }\n").unwrap();
    });
    let before_spawns = test_spawn_count();
    let stale = mediate_prepared_routine_execution(
        &source_fixture.context,
        &source_fixture.plan,
        source_prepared,
        Some(grant),
        RoutineCancellation::new(),
        RoutineReuseInput::default(),
    )
    .expect_err("mutated dirty bytes must invalidate the prepared snapshot");
    assert!(matches!(
        stale.cause(),
        "mediator-context-preflight-stale" | "mediator-dirty-snapshot-stale"
    ));
    assert_eq!(test_spawn_count(), before_spawns);

    let scope_fixture = fixture("mediator-scope-swap", true);
    let scope_prepared = prepared(&scope_fixture);
    let grant = issue_grant(&scope_prepared, "scope-swap-session", None);
    let scope = scope_fixture.repo.root().join("target/routine/syntax");
    let moved = scope_fixture
        .repo
        .root()
        .join("target/routine/syntax-moved");
    set_test_mediator_pre_spawn_hook(move || {
        fs::rename(&scope, moved).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink("compile", scope).unwrap();
    });
    let swapped = mediate_prepared_routine_execution(
        &scope_fixture.context,
        &scope_fixture.plan,
        scope_prepared,
        Some(grant),
        RoutineCancellation::new(),
        RoutineReuseInput::default(),
    )
    .expect_err("scope replacement must fail the second preflight");
    assert_eq!(swapped.cause(), "mediator-output-scope-not-directory");
    assert_eq!(test_spawn_count(), before_spawns);

    #[cfg(unix)]
    {
        let root_fixture = fixture("mediator-root-swap", true);
        let root_prepared = prepared(&root_fixture);
        let root_grant = issue_grant(&root_prepared, "root-swap-session", None);
        let root = root_fixture.repo.root().to_path_buf();
        let moved = root.with_extension("moved");
        let hook_root = root.clone();
        let hook_moved = moved.clone();
        set_test_mediator_pre_spawn_hook(move || {
            fs::rename(&hook_root, &hook_moved).unwrap();
            std::os::unix::fs::symlink(&hook_moved, &hook_root).unwrap();
        });
        let replaced = mediate_prepared_routine_execution(
            &root_fixture.context,
            &root_fixture.plan,
            root_prepared,
            Some(root_grant),
            RoutineCancellation::new(),
            RoutineReuseInput::default(),
        )
        .expect_err("working-directory replacement must fail closed");
        fs::remove_file(&root).unwrap();
        fs::rename(&moved, &root).unwrap();
        assert!(matches!(
            replaced.cause(),
            "mediator-context-preflight-stale"
                | "mediator-working-directory-identity-invalid"
                | "mediator-working-directory-not-canonical"
        ));
        assert_eq!(test_spawn_count(), before_spawns);
    }
}
