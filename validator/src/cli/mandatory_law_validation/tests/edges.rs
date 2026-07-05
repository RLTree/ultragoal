use super::super::*;
use crate::scheduler::{SchedulerConfig, TaskClass};
use serde_json::json;
use std::fs;
use std::path::PathBuf;

#[test]
fn mandatory_law_validation_parse_and_scheduler_edges_are_bounded() {
    assert!(
        parse(&[
            "mandatory-law-validation".into(),
            "--law".into(),
            "schema-valid".into(),
            "--jobs".into(),
            "many".into()
        ])
        .expect_err("invalid jobs")
        .contains("invalid numeric value for --jobs")
    );

    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("mandatory-law-jobs-zero");
    super::create_law_package(&root);
    let command = MandatoryLawValidationCommand {
        law: Some("schema-valid".to_string()),
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
fn mandatory_law_validation_failure_edges_and_external_claim_output_are_rejected() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("mandatory-law-edges");
    super::create_law_package(&root);
    let missing = validate(
        &root,
        Some("missing-law"),
        SchedulerConfig::from_jobs(Some(1)).expect("scheduler"),
    );
    assert!(
        missing
            .failures
            .iter()
            .any(|failure| failure.contains("mandatory_law_missing:missing-law"))
    );

    crate::json_boundary::write_json(&root.join(REGISTRY_REL), &json!({})).expect("bad registry");
    let no_laws = validate(
        &root,
        Some("schema-valid"),
        SchedulerConfig::from_jobs(Some(1)).expect("scheduler"),
    );
    assert!(
        no_laws
            .failures
            .iter()
            .any(|failure| failure == "mandatory_law_registry_missing_laws")
    );

    fs::remove_file(root.join(REGISTRY_REL)).expect("remove registry");
    let missing_registry = validate(
        &root,
        None,
        SchedulerConfig::from_jobs(Some(1)).expect("scheduler"),
    );
    assert!(
        missing_registry
            .failures
            .iter()
            .any(|failure| failure.contains(REGISTRY_REL))
    );

    super::create_law_package(&root);
    let receipt = root.join("absolute-mandatory-law.json");
    let command = MandatoryLawValidationCommand {
        law: Some("schema-valid".to_string()),
        receipt: receipt.clone(),
        jobs: Some(1),
    };
    let err = run(&root, &command).expect_err("absolute claim receipt rejected");
    assert!(err.contains("root-relative claim artifact path"), "{err}");
    assert!(!receipt.is_file());

    let dispatch = crate::parse_command(&[
        "mandatory-law".to_string(),
        "validation".to_string(),
        "--law".to_string(),
        "schema-valid".to_string(),
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
fn mandatory_law_failure_summary_is_bounded_and_counts_hidden_failures() {
    let failures = (0..14)
        .map(|index| format!("failure-{index}"))
        .collect::<Vec<_>>();
    let summary = claims::why_failed("fail", &failures);
    assert!(summary.contains("failure-11"), "{summary}");
    assert!(!summary.contains("failure-12"), "{summary}");
    assert!(summary.contains("and 2 more"), "{summary}");
}

#[test]
fn mandatory_law_runtime_metrics_report_resource_and_saturation_states() {
    let empty = runtime::from_metrics(&[], 7);
    assert_eq!(empty.cache_mode, "mandatory_law_no_cache");
    assert_eq!(empty.saturation_status, "no_scheduler_tasks_started");

    let worker_missing = runtime::from_metrics(&[metric(0, 1, 1, "a", "ra")], 7);
    assert_eq!(worker_missing.worker_count, 0);
    assert_eq!(
        worker_missing.saturation_status,
        "scheduler_worker_count_missing"
    );

    let mixed = runtime::from_metrics(&[metric(1, 2, 3, "a", "ra"), metric(2, 1, 1, "b", "rb")], 7);
    assert_eq!(mixed.cpu_ms, Some(20));
    assert_eq!(mixed.memory_bytes, Some(200));
    assert_eq!(mixed.io_bytes, Some(2000));
    assert_eq!(mixed.cache_mode, "mixed_mandatory_law_scheduler_modes");
    assert_eq!(
        mixed.resource_measurement_status,
        "mixed_mandatory_law_scheduler_modes"
    );
    assert_eq!(
        mixed.saturation_status,
        "queued_parallel_mandatory_law_validation"
    );
}

#[test]
fn mandatory_law_run_propagates_telemetry_and_receipt_write_errors() {
    let no_manifest =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("mandatory-law-no-manifest");
    super::create_law_package(&no_manifest);
    fs::remove_file(no_manifest.join("plugin-manifest-draft.json")).expect("remove manifest");
    let command = MandatoryLawValidationCommand {
        law: Some("schema-valid".to_string()),
        receipt: PathBuf::from(RECEIPT_REL),
        jobs: Some(1),
    };
    let err = run(&no_manifest, &command).expect_err("candidate digest required");
    assert!(err.contains("plugin-manifest-draft.json"), "{err}");
    fs::remove_dir_all(no_manifest).expect("cleanup no manifest");

    let blocked =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("mandatory-law-blocked");
    super::create_law_package(&blocked);
    fs::create_dir_all(blocked.join("validation_artifacts/mandatory-law"))
        .expect("blocked parent root");
    fs::write(
        blocked.join("validation_artifacts/mandatory-law/blocked-parent"),
        "blocked",
    )
    .expect("blocked path");
    let blocked_command = MandatoryLawValidationCommand {
        receipt: PathBuf::from("validation_artifacts/mandatory-law/blocked-parent/receipt.json"),
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
