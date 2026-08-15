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
    let plan_path = home.join("fit-plan.json");
    fs::write(&plan_path, &plan.stdout).unwrap();

    let ParseOutcome::Invocation(apply_invocation) = parse_args([
        "--json",
        "fit",
        "apply",
        "--plan",
        plan_path.to_str().unwrap(),
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
pub(crate) fn diagnose_reports_authenticated_device_drift_without_writes() {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    let repo = Repository::new("fit-device-drift-diagnosis");
    let home = repo.root.with_extension("fit-device-drift-home");
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
    let plan_path = home.join("fit-plan.json");
    fs::write(&plan_path, &plan.stdout).unwrap();
    let ParseOutcome::Invocation(apply_invocation) = parse_args([
        "--json",
        "fit",
        "apply",
        "--plan",
        plan_path.to_str().unwrap(),
        "--accept-plan",
        &plan_sha256,
    ])
    .unwrap() else {
        panic!("expected fit apply invocation")
    };
    let applied = execute_invocation_with_home(&repo.root, apply_invocation, Some(&home))
        .render(OutputMode::Json);
    assert_eq!(applied.exit_code, 0);

    fs::remove_file(repo.root.join("migration/authority-routes.json")).unwrap();
    let current_device = fs::metadata(&authority).unwrap().dev();
    crate::repository_fit::simulate_device_drift_for_test(&authority, current_device + 1).unwrap();
    let pending_probe = pending.join(format!("{}.pending.json", "a".repeat(64)));
    fs::write(&pending_probe, b"{}\n").unwrap();
    fs::set_permissions(&pending_probe, fs::Permissions::from_mode(0o600)).unwrap();
    let before_pending = tree(&home);
    let ParseOutcome::Invocation(invocation) = parse_args(["--json", "diagnose"]).unwrap() else {
        panic!("expected diagnosis invocation")
    };
    let pending_result =
        execute_invocation_with_home(&repo.root, invocation, Some(&home)).render(OutputMode::Json);
    assert_eq!(pending_result.exit_code, 1);
    let value: serde_json::Value = serde_json::from_slice(&pending_result.stdout).unwrap();
    assert_eq!(value["status"], "pending_recovery_present");
    assert_eq!(value["pending_envelope_count"], 1);
    assert!(value["quarantine_plan"].is_null());
    assert_eq!(value["migration_effect"], "none");
    assert_eq!(tree(&home), before_pending);
    fs::remove_file(&pending_probe).unwrap();

    let before_repo = tree(&repo.root);
    let before_home = tree(&home);
    let before_status = repo.status();

    let ParseOutcome::Invocation(invocation) = parse_args(["--json", "diagnose"]).unwrap() else {
        panic!("expected diagnosis invocation")
    };
    let streams =
        execute_invocation_with_home(&repo.root, invocation, Some(&home)).render(OutputMode::Json);
    assert_eq!(streams.exit_code, 1);
    assert!(streams.stderr.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&streams.stdout).unwrap();
    assert_eq!(
        value["schema_version"],
        "RepositoryFitAuthorityMigrationAdmission-v1"
    );
    assert_eq!(value["status"], "device_identity_changed");
    assert_eq!(value["authority_status"], "authenticated_device_only_drift");
    assert_eq!(value["reservation_count"], 1);
    assert_eq!(value["nonterminal_reservation_count"], 0);
    assert_eq!(value["pending_envelope_count"], 0);
    assert_eq!(value["process_lock_count"], 1);
    assert_eq!(value["stored_root"]["device"], current_device + 1);
    assert_eq!(value["current_root"]["device"], current_device);
    assert_eq!(
        value["stored_root"]["inode"],
        value["current_root"]["inode"]
    );
    assert_eq!(
        value["quarantine_plan"]["strategy"],
        "whole_owner_quarantine_then_fresh_bootstrap"
    );
    assert_eq!(
        value["quarantine_plan"]["apply_capability"],
        "not_implemented"
    );
    assert_eq!(value["migration_effect"], "none");
    assert_eq!(value["migration_authorized"], false);
    assert_eq!(value["claim_effect"], "none");
    let output = String::from_utf8(streams.stdout).unwrap();
    assert!(!output.contains(home.to_str().unwrap()));
    assert!(!output.contains(repo.root.to_str().unwrap()));
    assert_eq!(tree(&repo.root), before_repo);
    assert_eq!(tree(&home), before_home);
    assert_eq!(repo.status(), before_status);

    let ledger = authority.join("authority-ledger.json");
    let bytes = String::from_utf8(fs::read(&ledger).unwrap()).unwrap();
    assert!(bytes.contains("\"generation\":3"));
    fs::write(
        &ledger,
        bytes.replacen("\"generation\":3", "\"generation\":4", 1),
    )
    .unwrap();
    let before_invalid = tree(&home);
    let ParseOutcome::Invocation(invocation) = parse_args(["--json", "diagnose"]).unwrap() else {
        panic!("expected diagnosis invocation")
    };
    let invalid =
        execute_invocation_with_home(&repo.root, invocation, Some(&home)).render(OutputMode::Json);
    assert_eq!(invalid.exit_code, 1);
    let value: serde_json::Value = serde_json::from_slice(&invalid.stdout).unwrap();
    assert_eq!(value["status"], "authority_state_invalid");
    assert_eq!(value["authority_status"], "unavailable");
    assert!(value["quarantine_plan"].is_null());
    assert_eq!(value["migration_effect"], "none");
    assert_eq!(value["migration_authorized"], false);
    assert_eq!(tree(&home), before_invalid);

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
    let plan_path = home.join("fit-plan.json");
    fs::write(&plan_path, &plan.stdout).unwrap();

    crate::repository_fit::after_effect_before_terminal_for_test(|| {
        panic!("simulated public process interruption after the workspace effect")
    });
    let interrupted = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let ParseOutcome::Invocation(invocation) = parse_args([
            "--json",
            "fit",
            "apply",
            "--plan",
            plan_path.to_str().unwrap(),
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
        plan_path.to_str().unwrap(),
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
