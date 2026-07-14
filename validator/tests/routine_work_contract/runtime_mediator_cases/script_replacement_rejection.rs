use super::*;

#[test]
#[cfg(target_os = "macos")]
pub(crate) fn external_dash_script_replacement_cannot_execute_or_seed_reuse() {
    let _serial = mediator_lock();
    let fixture = fixture("mediator-external-dash-source", true);
    let owner = TempRepo::new("mediator-external-dash-source-owner");
    let command = owner.root().join("command.sh");
    fs::write(&command, b"exit 0\n").unwrap();
    let command_argument = command.to_string_lossy().into_owned();
    let first_prepared = prepared_with_arguments(
        &fixture,
        |_| vec![command_argument.clone()],
        10_000,
        1024 * 1024,
    );
    let first_grant = issue_grant(&first_prepared, "external-source-1", None);
    let replacement = owner.root().join("replacement.sh");
    fs::write(&replacement, command_file_script()).unwrap();
    fs::rename(&replacement, &command).unwrap();
    let before_spawns = test_spawn_count();
    let first = mediate(
        &fixture,
        first_prepared,
        Some(first_grant),
        RoutineCancellation::new(),
        Vec::new(),
    );
    assert_eq!(first.status(), RoutineMediatorStatus::IncompleteExecution);
    assert_eq!(
        first.nodes()[0].failure_code(),
        Some("MEDIATOR-CHECK-FAILED")
    );
    assert!(
        first
            .nodes()
            .iter()
            .all(|node| node.result_artifact_sha256().is_none())
    );
    assert!(first.reuse_artifacts().is_empty());
    assert_eq!(test_spawn_count(), before_spawns + 1);
    for node_id in ["syntax", "compile", "unit"] {
        assert!(
            !fixture
                .repo
                .root()
                .join(format!("target/routine/{node_id}/result.txt"))
                .exists(),
            "undeclared external source reached effect for {node_id}"
        );
    }

    fs::write(&command, b"exit 71\n").unwrap();
    let recovery_marker = first.recovery_marker().unwrap().to_owned();
    let retry_prepared = prepared_with_arguments(
        &fixture,
        |_| vec![command_argument.clone()],
        10_000,
        1024 * 1024,
    );
    let retry_grant = issue_grant(&retry_prepared, "external-source-2", Some(recovery_marker));
    let retry = mediate(
        &fixture,
        retry_prepared,
        Some(retry_grant),
        RoutineCancellation::new(),
        first.reuse_artifacts().to_vec(),
    );
    assert_eq!(retry.status(), RoutineMediatorStatus::IncompleteExecution);
    assert!(
        retry
            .nodes()
            .iter()
            .all(|node| node.disposition() != RoutineNodeDisposition::Reused)
    );
    assert!(retry.reuse_artifacts().is_empty());
    assert_eq!(test_spawn_count(), before_spawns + 2);
}

#[test]
#[cfg(target_os = "macos")]
pub(crate) fn bound_dash_source_executes_reuses_exactly_and_mutation_invalidates_reuse() {
    let _serial = mediator_lock();
    let fixture = fixture("mediator-bound-dash-source", true);
    let relative_source = "target/routine/command.sh";
    let source = fixture.repo.root().join(relative_source);
    fs::write(&source, command_file_script()).unwrap();
    let before_spawns = test_spawn_count();
    let first_prepared = prepared_with_read_source(
        &fixture,
        |_| vec![relative_source.to_owned()],
        relative_source,
        10_000,
        1024 * 1024,
    );
    let first_grant = issue_grant(&first_prepared, "bound-source-1", None);
    let first = mediate(
        &fixture,
        first_prepared,
        Some(first_grant),
        RoutineCancellation::new(),
        Vec::new(),
    );
    assert_eq!(first.status(), RoutineMediatorStatus::CompleteExecution);
    assert_eq!(first.reuse_artifacts().len(), 3);
    assert_eq!(test_spawn_count(), before_spawns + 3);

    let reuse_prepared = prepared_with_read_source(
        &fixture,
        |_| vec![relative_source.to_owned()],
        relative_source,
        10_000,
        1024 * 1024,
    );
    let reuse_grant = issue_grant(&reuse_prepared, "bound-source-2", None);
    let reused = mediate(
        &fixture,
        reuse_prepared,
        Some(reuse_grant),
        RoutineCancellation::new(),
        first.reuse_artifacts().to_vec(),
    );
    assert_eq!(reused.status(), RoutineMediatorStatus::CompleteExecution);
    assert!(
        reused
            .nodes()
            .iter()
            .all(|node| node.disposition() == RoutineNodeDisposition::Reused)
    );
    assert_eq!(test_spawn_count(), before_spawns + 3);

    fs::write(&source, b"exit 71\n").unwrap();
    let changed_prepared = prepared_with_read_source(
        &fixture,
        |_| vec![relative_source.to_owned()],
        relative_source,
        10_000,
        1024 * 1024,
    );
    let changed_grant = issue_grant(&changed_prepared, "bound-source-3", None);
    let changed = mediate(
        &fixture,
        changed_prepared,
        Some(changed_grant),
        RoutineCancellation::new(),
        first.reuse_artifacts().to_vec(),
    );
    assert_eq!(changed.status(), RoutineMediatorStatus::IncompleteExecution);
    assert_eq!(
        changed.nodes()[0].failure_code(),
        Some("MEDIATOR-CHECK-FAILED")
    );
    assert!(
        changed
            .nodes()
            .iter()
            .all(|node| node.disposition() != RoutineNodeDisposition::Reused)
    );
    assert!(changed.reuse_artifacts().is_empty());
    assert_eq!(test_spawn_count(), before_spawns + 4);
}
