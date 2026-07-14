use super::*;

#[cfg(target_vendor = "apple")]
#[test]
pub(crate) fn fit_apply_runs_the_public_production_route_and_retires_recovery_state() {
    use std::os::unix::fs::PermissionsExt;

    let repo = Repository::new("fit-apply-production");
    let home = repo.root.with_extension("fit-apply-home");
    let state = home.join(".codex/state/harness-ultragoal/repository-fit");
    let authority = state.join("authority");
    let pending = state.join("pending");
    fs::create_dir_all(&authority).unwrap();
    fs::create_dir_all(&pending).unwrap();
    for path in [
        &home,
        &home.join(".codex"),
        &home.join(".codex/state"),
        &home.join(".codex/state/harness-ultragoal"),
        &state,
        &authority,
        &pending,
    ] {
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let home = fs::canonicalize(&home).unwrap();

    let ParseOutcome::Invocation(plan_invocation) = parse_args(["--json", "fit", "plan"]).unwrap()
    else {
        panic!("expected fit plan invocation")
    };
    let plan = execute_invocation_with_home(&repo.root, plan_invocation, Some(&home))
        .render(OutputMode::Json);
    assert_eq!(plan.exit_code, 0);
    assert!(plan.stdout.ends_with(b"\n"));
    let plan_value: serde_json::Value = serde_json::from_slice(&plan.stdout).unwrap();
    let plan_sha256 = plan_value["plan"]["plan_sha256"]
        .as_str()
        .unwrap()
        .to_owned();
    fs::create_dir_all(repo.root.join("validation_artifacts")).unwrap();
    fs::write(
        repo.root.join("validation_artifacts/fit-plan.json"),
        &plan.stdout,
    )
    .unwrap();

    let ParseOutcome::Invocation(apply_invocation) = parse_args([
        "--json",
        "fit",
        "apply",
        "--plan",
        "validation_artifacts/fit-plan.json",
        "--accept-plan",
        &plan_sha256,
    ])
    .unwrap() else {
        panic!("expected fit apply invocation")
    };
    let applied = execute_invocation_with_home(&repo.root, apply_invocation, Some(&home))
        .render(OutputMode::Json);
    assert_eq!(
        applied.exit_code,
        0,
        "{}",
        String::from_utf8_lossy(&applied.stderr)
    );
    assert!(applied.stderr.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&applied.stdout).unwrap();
    assert_eq!(value["schema_version"], "RepositoryFitProductionOutcome-v1");
    assert_eq!(value["status"], "applied");
    assert_eq!(value["effect"], "workspace_write");

    let pending_names = fs::read_dir(&pending)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    assert_eq!(pending_names.len(), 1);
    assert!(pending_names[0].ends_with(".lock"));
    assert_eq!(fs::read_dir(&authority).unwrap().count(), 3);

    let ParseOutcome::Invocation(verify_invocation) =
        parse_args(["--json", "fit", "verify"]).unwrap()
    else {
        panic!("expected fit verify invocation")
    };
    let verified = execute_invocation_with_home(&repo.root, verify_invocation, Some(&home))
        .render(OutputMode::Json);
    assert_eq!(verified.exit_code, 0);
    let value: serde_json::Value = serde_json::from_slice(&verified.stdout).unwrap();
    assert_eq!(value["idempotent"], true);

    fs::remove_dir_all(home).unwrap();
}

#[cfg(target_vendor = "apple")]
#[test]
pub(crate) fn fit_apply_services_pending_recovery_before_a_conflicting_plan_can_block_it() {
    use std::os::unix::fs::PermissionsExt;

    let repo = Repository::new("fit-apply-recovery-before-plan");
    let home = repo.root.with_extension("fit-apply-recovery-home");
    let state = home.join(".codex/state/harness-ultragoal/repository-fit");
    let authority = state.join("authority");
    let pending = state.join("pending");
    fs::create_dir_all(&authority).unwrap();
    fs::create_dir_all(&pending).unwrap();
    for path in [
        &home,
        &home.join(".codex"),
        &home.join(".codex/state"),
        &home.join(".codex/state/harness-ultragoal"),
        &state,
        &authority,
        &pending,
    ] {
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let home = fs::canonicalize(&home).unwrap();

    let ParseOutcome::Invocation(plan_invocation) = parse_args(["--json", "fit", "plan"]).unwrap()
    else {
        panic!("expected fit plan invocation")
    };
    let plan = execute_invocation_with_home(&repo.root, plan_invocation, Some(&home))
        .render(OutputMode::Json);
    assert_eq!(plan.exit_code, 0);
    let plan_value: serde_json::Value = serde_json::from_slice(&plan.stdout).unwrap();
    let plan_sha256 = plan_value["plan"]["plan_sha256"]
        .as_str()
        .unwrap()
        .to_owned();
    fs::create_dir_all(repo.root.join("validation_artifacts")).unwrap();
    fs::write(
        repo.root.join("validation_artifacts/fit-plan.json"),
        &plan.stdout,
    )
    .unwrap();

    crate::repository_fit::after_effect_before_terminal_for_test(|| {
        panic!("simulated public process interruption after the workspace effect")
    });
    let interrupted = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let ParseOutcome::Invocation(invocation) = parse_args([
            "--json",
            "fit",
            "apply",
            "--plan",
            "validation_artifacts/fit-plan.json",
            "--accept-plan",
            &plan_sha256,
        ])
        .unwrap() else {
            panic!("expected fit apply invocation")
        };
        execute_invocation_with_home(&repo.root, invocation, Some(&home))
    }));
    assert!(interrupted.is_err());
    assert!(
        fs::read_to_string(authority.join("authority-ledger.json"))
            .unwrap()
            .contains("effect_started")
    );
    assert_eq!(fs::read_dir(&pending).unwrap().count(), 2);

    fs::write(repo.root.join("AGENTS.md"), b"conflicting user bytes\n").unwrap();
    let conflict = fs::read(repo.root.join("AGENTS.md")).unwrap();
    let ParseOutcome::Invocation(invocation) = parse_args([
        "--json",
        "fit",
        "apply",
        "--plan",
        "validation_artifacts/fit-plan.json",
        "--accept-plan",
        &plan_sha256,
    ])
    .unwrap() else {
        panic!("expected fit apply invocation")
    };
    let recovery =
        execute_invocation_with_home(&repo.root, invocation, Some(&home)).render(OutputMode::Json);
    assert_eq!(recovery.exit_code, 3);
    assert!(recovery.stderr.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&recovery.stdout).unwrap();
    assert_eq!(value["adapter_error_id"], "apply_lease_invalid");
    assert_eq!(value["ledger_state"], "effect_started");
    assert_eq!(value["effect_started"], true);
    assert_eq!(fs::read(repo.root.join("AGENTS.md")).unwrap(), conflict);
    assert_eq!(fs::read_dir(&pending).unwrap().count(), 2);

    fs::remove_dir_all(home).unwrap();
}
