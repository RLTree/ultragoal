use super::*;

#[test]
#[cfg(target_os = "macos")]
pub(crate) fn fabricated_or_mutated_reuse_never_becomes_a_cache_hit() {
    let _serial = mediator_lock();
    let fixture = fixture("mediator-reuse-forgery", true);
    let first_prepared = prepared(&fixture);
    let first_grant = issue_grant(&first_prepared, "reuse-forgery-1", None);
    let first = mediate(
        &fixture,
        first_prepared,
        Some(first_grant),
        RoutineCancellation::new(),
        Vec::new(),
    );
    assert_eq!(first.status(), RoutineMediatorStatus::CompleteExecution);
    let mut started: serde_json::Value =
        serde_json::from_slice(&first.reuse_artifacts()[0]).unwrap();
    started["state"] = serde_json::Value::String("started".to_owned());
    let started_prepared = prepared(&fixture);
    let started_grant = issue_grant(&started_prepared, "reuse-started", None);
    let before_started = test_spawn_count();
    let refusal = mediate_prepared_routine_execution(
        &fixture.context,
        &fixture.plan,
        started_prepared,
        Some(started_grant),
        RoutineCancellation::new(),
        RoutineReuseInput::new(vec![serde_json::to_vec(&started).unwrap()]),
    )
    .expect_err("started reuse evidence must never be retried silently");
    assert_eq!(refusal.cause(), "mediator-ambiguous-started-artifact");
    assert_eq!(test_spawn_count(), before_started);

    let mut artifacts = first.reuse_artifacts().to_vec();
    let offset = artifacts[0]
        .iter()
        .position(|byte| *byte == b'c')
        .expect("artifact has mutable payload byte");
    artifacts[0][offset] = b'd';
    artifacts.push(
        br#"{"schema_version":"RoutineMediatedReuseArtifact-v1","state":"complete"}"#.to_vec(),
    );
    let before_second = test_spawn_count();
    let second_prepared = prepared(&fixture);
    let second_grant = issue_grant(&second_prepared, "reuse-forgery-2", None);
    let second = mediate(
        &fixture,
        second_prepared,
        Some(second_grant),
        RoutineCancellation::new(),
        artifacts,
    );
    assert_eq!(second.status(), RoutineMediatorStatus::CompleteExecution);
    assert!(
        second
            .nodes()
            .iter()
            .all(|node| node.disposition() == RoutineNodeDisposition::Executed)
    );
    assert_eq!(test_spawn_count(), before_second + 3);

    let mut nested = first.reuse_artifacts().to_vec();
    let mut nested_value: serde_json::Value = serde_json::from_slice(&nested[1]).unwrap();
    nested_value["result_artifact"]["behavior_sha256"] =
        serde_json::Value::String(format!("sha256:{}", "e".repeat(64)));
    nested[1] = serde_json::to_vec(&nested_value).unwrap();
    let nested_prepared = prepared(&fixture);
    let nested_grant = issue_grant(&nested_prepared, "reuse-forgery-3", None);
    let before_nested = test_spawn_count();
    let nested_result = mediate(
        &fixture,
        nested_prepared,
        Some(nested_grant),
        RoutineCancellation::new(),
        nested,
    );
    assert_eq!(
        nested_result.status(),
        RoutineMediatorStatus::CompleteExecution
    );
    assert_eq!(
        nested_result
            .nodes()
            .iter()
            .map(|node| node.disposition())
            .collect::<Vec<_>>(),
        [
            RoutineNodeDisposition::Reused,
            RoutineNodeDisposition::Executed,
            RoutineNodeDisposition::Executed,
        ]
    );
    assert_eq!(test_spawn_count(), before_nested + 2);
}

#[test]
#[cfg(target_os = "macos")]
pub(crate) fn strict_expansion_and_capability_fallback_execute_the_selected_plan_exactly() {
    let _serial = mediator_lock();
    let strict = strict_fixture("mediator-strict");
    assert_eq!(
        strict
            .plan
            .checks()
            .iter()
            .map(|check| check.node_id())
            .collect::<Vec<_>>(),
        ["syntax", "compile", "unit"]
    );
    let strict_prepared = prepared(&strict);
    let strict_grant = issue_grant(&strict_prepared, "strict-session", None);
    let strict_result = mediate(
        &strict,
        strict_prepared,
        Some(strict_grant),
        RoutineCancellation::new(),
        Vec::new(),
    );
    assert_eq!(
        strict_result.status(),
        RoutineMediatorStatus::CompleteExecution
    );
    assert_eq!(strict_result.nodes().len(), 3);

    let fallback = fallback_fixture("mediator-fallback");
    let fallback_prepared = prepared(&fallback);
    assert_eq!(
        effect_request(&fallback_prepared).intents()[0].selected_tool(),
        "dash"
    );
    let fallback_grant = issue_grant(&fallback_prepared, "fallback-session", None);
    let fallback_result = mediate(
        &fallback,
        fallback_prepared,
        Some(fallback_grant),
        RoutineCancellation::new(),
        Vec::new(),
    );
    assert_eq!(
        fallback_result.status(),
        RoutineMediatorStatus::CompleteExecution
    );
    assert_eq!(fallback_result.nodes().len(), 1);
    assert_eq!(fallback_result.nodes()[0].node_id(), "compile");
}

#[test]
#[cfg(target_os = "macos")]
pub(crate) fn current_user_owned_0555_runner_is_rejected_before_spawn_or_effect() {
    const CHILD_ENV: &str = "HUL_ROUTINE_OWNED_EXECUTABLE_CHILD";
    const TEST_NAME: &str =
        "runtime_mediator::current_user_owned_0555_runner_is_rejected_before_spawn_or_effect";

    let _serial = mediator_lock();
    if let Some(expected) = std::env::var_os(CHILD_ENV) {
        let expected = PathBuf::from(expected).canonicalize().unwrap();
        let fixture = fixture("mediator-owned-0555-executable", true);
        let selected = fixture
            .context
            .capabilities()
            .tool("dash")
            .and_then(|tool| tool.executable.as_deref())
            .map(PathBuf::from)
            .unwrap();
        assert_eq!(selected, expected);
        let metadata = fs::symlink_metadata(&selected).unwrap();
        assert_eq!(metadata.uid(), unsafe { libc::geteuid() });
        assert_eq!(metadata.mode() & 0o777, 0o555);

        let prepared = prepared(&fixture);
        let grant = issue_grant(&prepared, "owned-0555-runner", None);
        let before_spawns = test_spawn_count();
        let error = mediate_prepared_routine_execution(
            &fixture.context,
            &fixture.plan,
            prepared,
            Some(grant),
            RoutineCancellation::new(),
            RoutineReuseInput::default(),
        )
        .expect_err("a current-user-owned 0555 runner remains chmod-mutable");
        assert_eq!(error.cause(), "mediator-executable-path-mutable");
        assert_eq!(test_spawn_count(), before_spawns);
        for node_id in ["syntax", "compile", "unit"] {
            assert!(
                !fixture
                    .repo
                    .root()
                    .join(format!("target/routine/{node_id}/result.txt"))
                    .exists(),
                "owned runner produced an effect for {node_id}"
            );
        }
        return;
    }

    let owner = TempRepo::new("mediator-owned-runner-parent");
    let bin = owner.root().join("owned-bin");
    let executable = bin.join("dash");
    fs::create_dir_all(&bin).unwrap();
    fs::copy("/bin/dash", &executable).unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o555)).unwrap();
    fs::set_permissions(&bin, fs::Permissions::from_mode(0o555)).unwrap();
    let path =
        std::env::join_paths([bin.as_path(), Path::new("/usr/bin"), Path::new("/bin")]).unwrap();
    let output = Command::new(std::env::current_exe().unwrap())
        .args([TEST_NAME, "--exact", "--nocapture", "--test-threads=1"])
        .env(CHILD_ENV, &executable)
        .env("PATH", path)
        .output()
        .unwrap();
    fs::set_permissions(&bin, fs::Permissions::from_mode(0o755)).unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(
        output.status.success(),
        "owned-runner child failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
