use serde_json::json;
use std::path::PathBuf;

mod surface_edges;

fn raw(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

fn root(label: &str, script: &str) -> PathBuf {
    let root = crate::self_tests::boundaries::support::temp_root(label);
    std::fs::create_dir_all(root.join("scripts")).expect("scripts");
    std::fs::write(root.join("scripts/check"), script).expect("script");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["scripts/check"]}),
    )
    .expect("manifest");
    root
}

#[test]
fn routine_help_fit_repo_and_target_repo_paths_are_discoverable() {
    let help = crate::cli::usage::text();
    for expected in [
        "Routine validation",
        "ultragoal routine check",
        "First plugin-activated repo path",
        "ultragoal fit-repo prove",
        "Target repo path",
        "ultragoal target-repo audit",
        "scripts/check is a narrow helper",
        "unsupported claims",
    ] {
        assert!(help.contains(expected), "{expected}");
    }
    assert!(matches!(
        crate::parse_command(&raw(&["--help"])).expect("help"),
        crate::Command::Help
    ));
    assert!(matches!(
        crate::parse_command(&raw(&["routine", "check"])).expect("routine"),
        crate::Command::Routine(_)
    ));
    assert!(matches!(
        crate::parse_command(&raw(&[
            "fit-repo",
            "prove",
            "--receipt-dir",
            "validation_artifacts/harness"
        ]))
        .expect("fit-repo"),
        crate::Command::Product(_)
    ));
    let target = crate::parse_command(&raw(&[
        "target-repo",
        "audit",
        "--surface-root",
        "fixtures/target-repo/valid-init",
        "--receipt",
        "validation_artifacts/cli/target-repo-audit-receipt.json",
    ]))
    .expect("target");
    let crate::Command::Audit {
        target_repo,
        receipt,
        ..
    } = target
    else {
        panic!("target-repo audit must route to source-local target audit");
    };
    assert_eq!(
        target_repo,
        Some(PathBuf::from("fixtures/target-repo/valid-init"))
    );
    assert_eq!(
        receipt,
        PathBuf::from("validation_artifacts/cli/target-repo-audit-receipt.json")
    );
}

#[test]
fn routine_command_writes_source_local_claim_ceiling_receipt() {
    let root = crate::self_tests::boundaries::support::temp_root("routine-command");
    std::fs::create_dir_all(root.join("scripts")).expect("scripts");
    std::fs::write(
        root.join("scripts/check"),
        "narrow-helper ceiling; unsupported claims; update_goal blocked",
    )
    .expect("script");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &serde_json::json!({"resources":["scripts/check"]}),
    )
    .expect("manifest");
    let receipt = PathBuf::from("validation_artifacts/cli/routine.json");
    let code = crate::command_run::run_with_exit_code(crate::Args {
        root: root.clone(),
        command: crate::Command::Routine(crate::cli::routine::RoutineCommand {
            receipt: receipt.clone(),
            target_repo: Some(PathBuf::from("target-repo")),
        }),
    })
    .expect("run routine");
    assert_eq!(code, 0);
    let value = crate::json_boundary::read_json(&root.join(receipt)).expect("receipt");
    assert_eq!(value["status"], "pass");
    assert_eq!(
        value["claim_ceiling"],
        "routine_usability_only_not_readiness"
    );
    assert!(
        value["blocked_claim_classes"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("update_goal_eligibility"))
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn routine_receipt_passes_for_discoverable_entrypoints_and_narrow_script() {
    let root = root(
        "routine-pass",
        "narrow-helper ceiling; unsupported claims; readiness blocked",
    );
    let command = crate::cli::routine::RoutineCommand {
        receipt: PathBuf::from("validation_artifacts/cli/routine.json"),
        target_repo: Some(PathBuf::from("target-repo")),
    };
    let value = crate::cli::routine::receipt(&root, &command).expect("receipt");
    assert_eq!(value["status"], "pass");
    assert_eq!(
        value["supported_claim_classes"],
        json!(["routine_usability"])
    );
    assert!(
        value["blocked_claim_classes"]
            .as_array()
            .unwrap()
            .contains(&json!("readiness"))
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn routine_parse_reads_receipt_and_target_repo_flags() {
    let command = crate::cli::routine::parse(&raw(&[
        "routine",
        "check",
        "--receipt",
        "validation_artifacts/cli/custom-routine.json",
        "--target-repo",
        "fixtures/target-repo/valid-init",
    ]))
    .expect("parse")
    .expect("routine command");
    assert_eq!(
        command.receipt,
        PathBuf::from("validation_artifacts/cli/custom-routine.json")
    );
    assert_eq!(
        command.target_repo,
        Some(PathBuf::from("fixtures/target-repo/valid-init"))
    );
}

#[test]
fn routine_run_covers_fail_status_and_error_paths() {
    let fail_root = root("routine-run-fail", "no delegation or ceiling");
    let fail_code = crate::cli::routine::run(
        &fail_root,
        &crate::cli::routine::RoutineCommand {
            receipt: PathBuf::from("validation_artifacts/cli/routine.json"),
            target_repo: None,
        },
    )
    .expect("fail receipt still writes");
    assert_eq!(fail_code, 1);
    std::fs::remove_dir_all(fail_root).expect("cleanup fail");

    let missing_manifest = crate::self_tests::boundaries::support::temp_root("routine-no-manifest");
    std::fs::create_dir_all(missing_manifest.join("scripts")).expect("scripts");
    std::fs::write(missing_manifest.join("scripts/check"), "routine check").expect("script");
    let receipt_error = crate::cli::routine::run(
        &missing_manifest,
        &crate::cli::routine::RoutineCommand {
            receipt: PathBuf::from("validation_artifacts/cli/routine.json"),
            target_repo: None,
        },
    )
    .expect_err("package digest failure propagates");
    assert!(
        receipt_error.contains("plugin-manifest-draft.json"),
        "{receipt_error}"
    );
    std::fs::remove_dir_all(missing_manifest).expect("cleanup missing manifest");

    let absolute_root = root("routine-absolute-receipt", "routine check");
    let absolute_error = crate::cli::routine::run(
        &absolute_root,
        &crate::cli::routine::RoutineCommand {
            receipt: PathBuf::from("/tmp/routine.json"),
            target_repo: None,
        },
    )
    .expect_err("absolute output path propagates");
    assert!(absolute_error.contains("root-relative"), "{absolute_error}");
    std::fs::remove_dir_all(absolute_root).expect("cleanup absolute");

    let traversal_root = root("routine-traversal-receipt", "routine check");
    let traversal_error = crate::cli::routine::run(
        &traversal_root,
        &crate::cli::routine::RoutineCommand {
            receipt: PathBuf::from("validation_artifacts/../routine.json"),
            target_repo: None,
        },
    )
    .expect_err("traversal output path propagates");
    assert!(
        traversal_error.contains("escapes root"),
        "{traversal_error}"
    );
    std::fs::remove_dir_all(traversal_root).expect("cleanup traversal");

    let write_root = root("routine-write-error", "routine check");
    std::fs::write(write_root.join("validation_artifacts"), "not a directory")
        .expect("blocked parent");
    let write_error = crate::cli::routine::run(
        &write_root,
        &crate::cli::routine::RoutineCommand {
            receipt: PathBuf::from("validation_artifacts/cli/routine.json"),
            target_repo: None,
        },
    )
    .expect_err("write error propagates");
    assert!(
        write_error.contains("create parent failed"),
        "{write_error}"
    );
    std::fs::remove_dir_all(write_root).expect("cleanup write");
}
