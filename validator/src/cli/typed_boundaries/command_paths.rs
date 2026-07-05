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
            "name": "typed-boundaries-command-test",
            "version": "0.0.0",
            "resources": []
        }),
    );
    root
}

#[test]
fn typed_boundaries_parser_requires_strict_product_command() {
    let raw = args(&[
        "typed-boundaries",
        "check",
        "--strict",
        "--receipt",
        "validation_artifacts/typed-boundaries/typed-boundaries.json",
        "--jobs",
        "3",
    ]);
    assert!(matches!(
        crate::parse_command(&raw).expect("command"),
        crate::Command::TypedBoundaries(_)
    ));
    let command = super::parse(&raw)
        .expect("typed-boundaries parse")
        .expect("typed-boundaries command");
    assert_eq!(
        command.receipt,
        std::path::PathBuf::from("validation_artifacts/typed-boundaries/typed-boundaries.json")
    );
    assert_eq!(command.jobs, Some(3));
    assert!(
        super::parse(&args(&["typed-boundaries", "check"]))
            .expect_err("strict required")
            .contains("requires --strict")
    );
    assert!(
        super::parse(&args(&[
            "typed-boundaries",
            "check",
            "--strict",
            "--jobs",
            "bad"
        ]))
        .expect_err("bad jobs")
        .contains("invalid numeric value")
    );
    assert!(
        super::parse(&args(&[
            "typed-boundaries",
            "check",
            "--strict",
            "--unknown"
        ]))
        .expect_err("unknown")
        .contains("unknown typed-boundaries check argument")
    );
    let defaults = super::parse(&args(&["typed-boundaries", "check", "--strict"]))
        .expect("default parse")
        .expect("typed-boundaries command");
    assert_eq!(
        defaults.receipt,
        std::path::PathBuf::from("validation_artifacts/observability/typed-boundaries-check.json")
    );
    assert_eq!(defaults.jobs, None);
    assert!(
        super::parse(&args(&[
            "typed-boundaries",
            "check",
            "--strict",
            "--receipt"
        ]))
        .expect_err("missing receipt")
        .contains("missing value for --receipt")
    );
}

#[test]
fn typed_boundaries_dispatch_writes_fail_closed_receipt() {
    let root = minimal_root("typed-boundaries-dispatch");
    let receipt =
        std::path::PathBuf::from("validation_artifacts/typed-boundaries/typed-boundaries.json");
    let command = crate::parse_command(&args(&[
        "typed-boundaries",
        "check",
        "--strict",
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
        crate::output_path::claim_artifact_path(&root, &receipt, "test typed boundaries receipt")
            .expect("typed receipt path");
    let value = crate::json_boundary::read_json(&receipt_path).expect("receipt");
    assert_eq!(value["status"], "fail");
    assert_eq!(value["operation"], "typed-boundaries.check");
    assert_eq!(
        value["candidate_digest"],
        crate::package::inventory::package_digest(&root).unwrap()
    );
    assert_eq!(
        value["claim_impact"],
        "typed_boundary_check_failed_blocks_readiness_release_completion_update_goal"
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn typed_boundaries_run_passes_for_current_authority_surface() {
    let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let receipt = std::path::PathBuf::from(
        "validation_artifacts/typed-boundaries/typed-boundaries-pass.json",
    );
    let receipt_path =
        crate::output_path::claim_artifact_path(&root, &receipt, "test typed boundaries receipt")
            .expect("typed receipt path");
    let _ = std::fs::remove_file(&receipt_path);
    let exit = super::run(
        &root,
        &super::TypedBoundariesCommand {
            receipt: receipt.clone(),
            jobs: Some(1),
        },
    )
    .expect("current typed-boundaries pass");
    assert_eq!(exit, 0);
    let value = crate::json_boundary::read_json(&receipt_path).expect("receipt");
    assert_eq!(value["status"], "pass");
    assert_eq!(value["failure_class"], "none");
    assert_eq!(value["where_failed"], "none");
    assert_eq!(
        value["claim_impact"],
        "supports_typed_boundary_check_source_local_observability_only"
    );
    let _ = std::fs::remove_file(receipt_path);
}

#[test]
fn typed_boundaries_run_reports_candidate_digest_error_before_receipt_claim() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "typed-boundaries-missing-candidate",
    );
    std::fs::create_dir_all(&root).expect("root");
    let command = super::TypedBoundariesCommand {
        receipt: std::path::PathBuf::from(
            "validation_artifacts/typed-boundaries/typed-boundaries.json",
        ),
        jobs: Some(1),
    };
    let err = super::run(&root, &command).expect_err("candidate digest is required");
    assert!(err.contains("plugin-manifest-draft.json"), "{err}");
    let receipt_path =
        crate::output_path::claim_artifact_path(&root, &command.receipt, "test typed boundaries")
            .expect("typed receipt path");
    assert!(!receipt_path.exists());
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn typed_boundaries_output_rejects_external_claim_paths() {
    let root = minimal_root("typed-boundaries-output-path");
    let command = super::TypedBoundariesCommand {
        receipt: std::path::PathBuf::from("/tmp/typed-boundaries.json"),
        jobs: Some(1),
    };
    let err = super::run(&root, &command).expect_err("absolute receipt rejected");
    assert!(err.contains("external debug only"), "{err}");
    std::fs::remove_dir_all(root).expect("cleanup");
}
