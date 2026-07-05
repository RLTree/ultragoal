use super::super::*;
use crate::scheduler::TaskClass;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
#[cfg(unix)]
use std::{ffi::OsString, os::unix::ffi::OsStringExt};

#[test]
fn schema_validation_parse_and_scheduler_edges_are_bounded() {
    assert!(
        parse(&[
            "schema-validation".into(),
            "--schema".into(),
            "test.schema.json".into(),
            "--file".into(),
            "sample.json".into(),
            "--jobs".into(),
            "many".into()
        ])
        .expect_err("invalid jobs")
        .contains("invalid numeric value for --jobs")
    );

    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("schema-jobs-zero");
    super::create_schema_package(&root, json!({"name": "Tree"}));
    let command = SchemaValidationCommand {
        schema: Some("test.schema.json".to_string()),
        file: Some(PathBuf::from("sample.json")),
        receipt: PathBuf::from(RECEIPT_REL),
        jobs: Some(0),
    };
    assert!(
        run(&root, &command)
            .expect_err("bounded scheduler")
            .contains("scheduler jobs must be at least 1")
    );
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn schema_validation_failure_edges_external_claim_output_and_default_run_are_observable() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("schema-validation-edges");
    super::create_schema_package(&root, json!({"name": "Tree"}));
    let store = crate::schema_catalog::load(&root);
    let escaped = validate_one(
        &root,
        &store,
        "test.schema.json",
        Path::new("../outside.json"),
    );
    assert!(
        escaped
            .iter()
            .any(|failure| failure.contains("package_path_invalid"))
    );
    let missing = validate_one(&root, &store, "test.schema.json", Path::new("missing.json"));
    assert!(
        missing
            .iter()
            .any(|failure| failure.contains("json_load_failed"))
    );
    #[cfg(unix)]
    {
        let invalid_utf8 = PathBuf::from(OsString::from_vec(vec![0xff]));
        let failures = validate_one(&root, &store, "test.schema.json", &invalid_utf8);
        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("non-utf8 path")),
            "{failures:?}"
        );
    }

    fs::remove_file(root.join("schemas/test.schema.json")).expect("remove schema");
    let bad_store = crate::schema_catalog::load(&root);
    let bootstrap = bootstrap_failures(&bad_store);
    assert!(
        bootstrap
            .iter()
            .any(|failure| failure.contains("schema_bootstrap_failed"))
    );

    super::create_schema_package(&root, json!({"name": "Tree"}));
    let receipt = root.join("absolute-schema-validation.json");
    let command = SchemaValidationCommand {
        schema: None,
        file: None,
        receipt: receipt.clone(),
        jobs: Some(1),
    };
    let err = run(&root, &command).expect_err("absolute claim receipt rejected");
    assert!(err.contains("root-relative claim artifact path"), "{err}");
    assert!(!receipt.is_file());

    super::create_schema_package(&root, json!({"name": "Tree"}));
    let relative_receipt = PathBuf::from(RECEIPT_REL);
    let command = SchemaValidationCommand {
        schema: None,
        file: None,
        receipt: relative_receipt.clone(),
        jobs: Some(1),
    };
    assert_eq!(run(&root, &command).expect("default run"), 1);
    let observation =
        crate::json_boundary::read_json(&root.join(relative_receipt)).expect("receipt");
    assert_eq!(observation["status"], "fail");
    assert_eq!(
        observation["event"]["repair_anchor_before"],
        "schema_validation_command_start"
    );

    super::create_schema_package(&root, json!({"name": "Tree"}));
    let dispatch = crate::parse_command(&[
        "schema".to_string(),
        "validation".to_string(),
        "--schema".to_string(),
        "test.schema.json".to_string(),
        "--file".to_string(),
        "sample.json".to_string(),
    ])
    .expect("parse dispatch");
    let code = crate::command_run::run_with_exit_code(crate::Args {
        root: root.clone(),
        command: dispatch,
    })
    .expect("dispatch run");
    assert_eq!(code, 0);
    fs::remove_dir_all(root).expect("cleanup edges");
}

#[test]
fn schema_validation_runtime_metrics_report_resource_and_saturation_states() {
    let empty = runtime::from_metrics(&[], 7, "before");
    assert_eq!(empty.cache_mode, "schema_validation_no_cache");
    assert_eq!(empty.saturation_status, "no_scheduler_tasks_started");
    assert_eq!(empty.repair_anchor_before, "before");

    let worker_missing = runtime::from_metrics(&[metric(0, 1, 1, "a", "ra")], 7, "before");
    assert_eq!(worker_missing.worker_count, 0);
    assert_eq!(
        worker_missing.saturation_status,
        "scheduler_worker_count_missing"
    );

    let mixed = runtime::from_metrics(
        &[metric(1, 2, 3, "a", "ra"), metric(2, 1, 1, "b", "rb")],
        7,
        "before",
    );
    assert_eq!(mixed.cpu_ms, Some(20));
    assert_eq!(mixed.memory_bytes, Some(200));
    assert_eq!(mixed.io_bytes, Some(2000));
    assert_eq!(mixed.cache_mode, "mixed_schema_validation_scheduler_modes");
    assert_eq!(
        mixed.resource_measurement_status,
        "mixed_schema_validation_scheduler_modes"
    );
    assert_eq!(mixed.saturation_status, "queued_parallel_schema_validation");
}

#[test]
fn schema_validation_run_propagates_telemetry_and_receipt_write_errors() {
    let no_manifest = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "schema-validation-no-manifest",
    );
    super::create_schema_package(&no_manifest, json!({"name": "Tree"}));
    fs::remove_file(no_manifest.join("plugin-manifest-draft.json")).expect("remove manifest");
    let command = SchemaValidationCommand {
        schema: Some("test.schema.json".to_string()),
        file: Some(PathBuf::from("sample.json")),
        receipt: PathBuf::from(RECEIPT_REL),
        jobs: Some(1),
    };
    let err = run(&no_manifest, &command).expect_err("candidate digest required");
    assert!(err.contains("plugin-manifest-draft.json"), "{err}");
    fs::remove_dir_all(no_manifest).expect("cleanup no manifest");

    let blocked =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("schema-validation-blocked");
    super::create_schema_package(&blocked, json!({"name": "Tree"}));
    fs::create_dir_all(blocked.join("validation_artifacts/schema-validation"))
        .expect("blocked parent root");
    fs::write(
        blocked.join("validation_artifacts/schema-validation/blocked-parent"),
        "blocked",
    )
    .expect("blocked path");
    let blocked_command = SchemaValidationCommand {
        receipt: PathBuf::from(
            "validation_artifacts/schema-validation/blocked-parent/receipt.json",
        ),
        ..command
    };
    let err = run(&blocked, &blocked_command).expect_err("receipt write should fail");
    assert!(err.contains("create parent failed"), "{err}");
    fs::remove_dir_all(blocked).expect("cleanup blocked");
}

fn metric(
    worker_count: usize,
    task_count: usize,
    queue_depth: usize,
    cache_mode: &'static str,
    resource_status: &'static str,
) -> crate::scheduler::Metrics {
    crate::scheduler::Metrics {
        task_class: TaskClass::PureReadParallel.id(),
        worker_count,
        task_count,
        queue_depth,
        wall_ms: 1,
        cpu_ms: Some(10),
        memory_bytes: Some(100),
        io_bytes: Some(1000),
        cache_mode,
        resource_measurement_status: resource_status,
        deterministic_ordering: true,
        shared_validation_artifact_writes_allowed: false,
    }
}
