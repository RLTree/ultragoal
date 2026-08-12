use super::*;

#[test]
pub(crate) fn absent_observe_store_is_empty_and_zero_write() {
    let repo = Repository::new("observe-absent");
    let home = disposable_home("observe-absent");
    let before_tree = tree(&repo.root);
    let before_status = repo.status();
    let before_home = tree(&home);
    let ParseOutcome::Invocation(invocation) = parse_args(["--json", "observe", "query"]).unwrap()
    else {
        panic!("expected invocation")
    };
    let streams =
        execute_invocation_with_home(&repo.root, invocation, Some(&home)).render(OutputMode::Json);
    assert_eq!(streams.exit_code, 0);
    let value: serde_json::Value = serde_json::from_slice(&streams.stdout).unwrap();
    assert_eq!(value["store_status"], "absent");
    assert_eq!(value["event_count"], 0);
    assert_eq!(value["causal_status"], "not_evaluated");
    assert_eq!(tree(&repo.root), before_tree);
    assert_eq!(repo.status(), before_status);
    assert_eq!(tree(&home), before_home);
    fs::remove_dir_all(home).unwrap();
}

#[cfg(unix)]
#[test]
pub(crate) fn substituted_host_state_parent_fails_closed_without_path_echo() {
    let repo = Repository::new("observe-host-symlink");
    let home = disposable_home("observe-host-symlink");
    let outside = repo.root.with_extension("outside-observe-host");
    fs::create_dir_all(&outside).unwrap();
    std::os::unix::fs::symlink(&outside, home.join(".codex")).unwrap();
    let before_tree = tree(&repo.root);
    let before_status = repo.status();
    let before_home = tree(&home);
    let ParseOutcome::Invocation(invocation) = parse_args(["--json", "observe", "query"]).unwrap()
    else {
        panic!("expected invocation")
    };
    let streams =
        execute_invocation_with_home(&repo.root, invocation, Some(&home)).render(OutputMode::Json);
    assert_eq!(streams.exit_code, 4);
    assert!(streams.stdout.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&streams.stderr).unwrap();
    assert_eq!(
        value["diagnostic_id"],
        "successor_runtime_observability_unavailable"
    );
    assert!(!String::from_utf8_lossy(&streams.stderr).contains("outside-observe-host"));
    assert_eq!(tree(&repo.root), before_tree);
    assert_eq!(repo.status(), before_status);
    assert_eq!(tree(&home), before_home);
    fs::remove_dir_all(home).unwrap();
    fs::remove_dir_all(outside).unwrap();
}

#[cfg(unix)]
#[test]
pub(crate) fn legacy_target_spool_is_ignored_without_bootstrap_or_writes() {
    let repo = Repository::new("observe-legacy-spool");
    let home = disposable_home("observe-legacy-spool");
    let legacy = repo.root.join("validation_artifacts/observability/spool");
    fs::create_dir_all(&legacy).unwrap();
    fs::write(legacy.join("legacy.jsonl"), b"legacy bytes\n").unwrap();
    let before_tree = tree(&repo.root);
    let before_status = repo.status();
    let before_home = tree(&home);
    let ParseOutcome::Invocation(invocation) = parse_args(["--json", "observe", "query"]).unwrap()
    else {
        panic!("expected invocation")
    };
    let streams =
        execute_invocation_with_home(&repo.root, invocation, Some(&home)).render(OutputMode::Json);
    assert_eq!(streams.exit_code, 0);
    let value: serde_json::Value = serde_json::from_slice(&streams.stdout).unwrap();
    assert_eq!(value["store_status"], "absent");
    assert_eq!(value["event_count"], 0);
    assert_eq!(tree(&repo.root), before_tree);
    assert_eq!(repo.status(), before_status);
    assert_eq!(tree(&home), before_home);
    fs::remove_dir_all(home).unwrap();
}

#[test]
pub(crate) fn unavailable_export_route_fails_before_repository_access_or_effect() {
    let canary = Path::new("/missing/successor-observe-export-canary-1297");
    let ParseOutcome::Invocation(invocation) = parse_args([
        "--json",
        "observe",
        "export",
        "--output",
        "out.json",
        "--approve-export",
    ])
    .unwrap() else {
        panic!("expected invocation")
    };
    let streams = execute_invocation(canary, invocation).render(OutputMode::Json);
    assert_eq!(streams.exit_code, 3);
    assert!(streams.stdout.is_empty());
    let text = String::from_utf8(streams.stderr).unwrap();
    assert!(text.contains("successor_runtime_authority_required"));
    assert!(!text.contains("successor-observe-export-canary-1297"));
    assert!(!text.contains("out.json"));
}

#[test]
pub(crate) fn incomplete_compatibility_inventory_refuses_without_writes() {
    let repo = Repository::new("reads");
    fs::write(repo.root.join("dirty-canary"), b"dirty\n").unwrap();
    let before_tree = tree(&repo.root);
    let before_status = repo.status();

    let ParseOutcome::Invocation(invocation) =
        parse_args(["--json", "inspect", "inventory"]).unwrap()
    else {
        panic!("expected invocation")
    };
    let streams = execute_invocation(&repo.root, invocation).render(OutputMode::Json);
    assert_eq!(streams.exit_code, 4);
    assert!(streams.stdout.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&streams.stderr).unwrap();
    assert_eq!(
        value["diagnostic_id"],
        "successor_runtime_inventory_unavailable"
    );

    assert_eq!(tree(&repo.root), before_tree);
    assert_eq!(repo.status(), before_status);
}
