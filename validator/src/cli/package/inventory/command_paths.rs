use serde_json::json;
use std::path::Path;

fn args(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| item.to_string()).collect()
}

fn write_json(path: &Path, value: &serde_json::Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("json write");
}

fn minimal_root(label: &str) -> std::path::PathBuf {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(label);
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({
            "name": "package-inventory-command-test",
            "version": "0.0.0",
            "resources": []
        }),
    );
    root
}

#[test]
fn package_inventory_parser_dispatches_to_product_command() {
    let raw = args(&[
        "package",
        "inventory",
        "--receipt",
        "validation_artifacts/package/package-inventory.json",
        "--jobs",
        "2",
    ]);
    assert!(matches!(
        crate::parse_command(&raw).expect("command"),
        crate::Command::PackageInventory(_)
    ));
    let command = super::parse(&raw)
        .expect("package inventory parse")
        .expect("package inventory command");
    assert_eq!(
        command.receipt,
        std::path::PathBuf::from("validation_artifacts/package/package-inventory.json")
    );
    assert_eq!(command.jobs, Some(2));
    assert!(matches!(
        super::parse(&args(&["package", "digest"])),
        Ok(None)
    ));
    assert!(
        super::parse(&args(&["package", "inventory", "--jobs", "bad"]))
            .expect_err("bad jobs")
            .contains("invalid numeric value")
    );
    assert!(
        super::parse(&args(&["package", "inventory", "--unknown"]))
            .expect_err("unknown")
            .contains("unknown package inventory argument")
    );
    let defaults = super::parse(&args(&["package", "inventory"]))
        .expect("default parse")
        .expect("inventory command");
    assert_eq!(
        defaults.receipt,
        std::path::PathBuf::from("validation_artifacts/observability/package-inventory.json")
    );
    assert_eq!(defaults.jobs, None);
    assert!(
        super::parse(&args(&["package", "inventory", "--receipt"]))
            .expect_err("missing receipt")
            .contains("missing value for --receipt")
    );
}

#[test]
fn package_inventory_dispatch_writes_fail_closed_receipt() {
    let root = minimal_root("package-inventory-dispatch");
    let receipt = std::path::PathBuf::from("validation_artifacts/package/package-inventory.json");
    let command = crate::parse_command(&args(&[
        "package",
        "inventory",
        "--receipt",
        receipt.to_str().expect("receipt str"),
        "--jobs",
        "1",
    ]))
    .expect("command");
    let exit = crate::command_run::run(crate::Args {
        root: root.clone(),
        command,
    })
    .expect("command run");
    assert_eq!(exit, 1);
    let receipt_path =
        crate::output_path::claim_artifact_path(&root, &receipt, "test package inventory receipt")
            .expect("typed receipt path");
    let value = crate::json_boundary::read_json(&receipt_path).expect("receipt");
    assert_eq!(value["status"], "fail");
    assert_eq!(value["operation"], "package.inventory");
    assert_eq!(
        value["candidate_digest"],
        crate::package::inventory::package_digest(&root).unwrap()
    );
    assert_eq!(
        value["claim_impact"],
        "package_inventory_failed_blocks_package_readiness_release_completion_update_goal"
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn package_inventory_run_passes_for_current_package_surface() {
    let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let receipt =
        std::path::PathBuf::from("validation_artifacts/package/package-inventory-pass.json");
    let receipt_path =
        crate::output_path::claim_artifact_path(&root, &receipt, "test package inventory receipt")
            .expect("typed receipt path");
    let _ = std::fs::remove_file(&receipt_path);
    let exit = super::run(
        &root,
        &super::PackageInventoryCommand {
            receipt: receipt.clone(),
            jobs: Some(1),
        },
    )
    .expect("current package inventory pass");
    assert_eq!(exit, 0);
    let value = crate::json_boundary::read_json(&receipt_path).expect("receipt");
    assert_eq!(value["status"], "pass");
    assert_eq!(value["failure_class"], "none");
    assert_eq!(value["where_failed"], "none");
    assert_eq!(
        value["claim_impact"],
        "supports_package_inventory_source_local_observability_only"
    );
    let _ = std::fs::remove_file(receipt_path);
}

#[test]
fn package_inventory_run_reports_candidate_digest_error_before_receipt_claim() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "package-inventory-missing-candidate",
    );
    std::fs::create_dir_all(&root).expect("root");
    let command = super::PackageInventoryCommand {
        receipt: std::path::PathBuf::from("validation_artifacts/package/package-inventory.json"),
        jobs: Some(1),
    };
    let err = super::run(&root, &command).expect_err("candidate digest is required");
    assert!(err.contains("plugin-manifest-draft.json"), "{err}");
    let receipt_path = crate::output_path::claim_artifact_path(
        &root,
        &command.receipt,
        "test package inventory receipt",
    )
    .expect("typed receipt path");
    assert!(!receipt_path.exists());
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn package_inventory_run_rejects_zero_workers_before_receipt_claim() {
    let root = minimal_root("package-inventory-zero-workers");
    let command = super::PackageInventoryCommand {
        receipt: std::path::PathBuf::from("validation_artifacts/package/package-inventory.json"),
        jobs: Some(0),
    };
    let err = super::run(&root, &command).expect_err("zero workers rejected");
    assert!(err.contains("scheduler jobs must be at least 1"), "{err}");
    let receipt_path = crate::output_path::claim_artifact_path(
        &root,
        &command.receipt,
        "test package inventory receipt",
    )
    .expect("typed receipt path");
    assert!(!receipt_path.exists());
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn package_inventory_scheduler_reports_final_bytecode_residue() {
    let root = minimal_root("package-inventory-final-bytecode");
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({
            "name": "package-inventory-final-bytecode",
            "version": "0.0.0",
            "resources": ["plugin-manifest-draft.json"]
        }),
    );
    std::fs::create_dir_all(root.join("__pycache__")).expect("bytecode dir");
    std::fs::write(root.join("__pycache__/stale.pyc"), "bytecode").expect("bytecode");
    let scheduled = super::run_inventory_tasks(
        &root,
        crate::scheduler::SchedulerConfig::from_jobs(Some(1)).expect("scheduler"),
    );
    let failures = scheduled.values.into_iter().flatten().collect::<Vec<_>>();
    assert!(
        failures.iter().any(|failure| failure
            .contains("plugin-inventory-closure:generated bytecode not allowed after validation")),
        "{failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn package_inventory_output_rejects_external_claim_paths() {
    let root = minimal_root("package-inventory-output-path");
    let command = super::PackageInventoryCommand {
        receipt: std::path::PathBuf::from("/tmp/package-inventory.json"),
        jobs: Some(1),
    };
    let err = super::run(&root, &command).expect_err("absolute receipt rejected");
    assert!(err.contains("external debug only"), "{err}");
    std::fs::remove_dir_all(root).expect("cleanup");
}
