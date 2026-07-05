use super::super::*;
use crate::scheduler::{SchedulerConfig, TaskClass};
use serde_json::json;
use std::fs;
use std::path::PathBuf;

#[test]
fn foundational_trace_parse_rejects_argument_edges() {
    let default = parse(&[
        "foundational-trace".into(),
        "check".into(),
        "--strict".into(),
    ])
    .expect("default parse")
    .expect("command");
    assert!(default.jobs.is_none());

    assert!(
        parse(&[
            "foundational-trace".into(),
            "check".into(),
            "--strict".into(),
            "--unknown".into()
        ])
        .expect_err("unknown")
        .contains("unknown foundational-trace check argument")
    );
    assert!(
        parse(&[
            "foundational-trace".into(),
            "check".into(),
            "--strict".into(),
            "--receipt".into()
        ])
        .expect_err("missing value")
        .contains("missing value for --receipt")
    );
    assert!(
        parse(&[
            "foundational-trace".into(),
            "check".into(),
            "--strict".into(),
            "--jobs".into(),
            "many".into()
        ])
        .expect_err("invalid jobs")
        .contains("invalid numeric value for --jobs")
    );
}

#[test]
fn foundational_trace_failure_edges_and_external_claim_output_are_rejected() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("foundational-trace-edges");
    super::write_minimal_root(&root, &["agent-queryable-observability"], true);
    let missing = validate(
        &root,
        Some("missing-obligation"),
        SchedulerConfig::from_jobs(Some(1)).expect("scheduler"),
    );
    assert!(missing.failures.iter().any(|failure| {
        failure.contains("foundational_law_trace_missing_obligation:missing-obligation")
    }));

    fs::remove_file(root.join(TRACE_REL)).expect("remove trace");
    let no_trace = validate(
        &root,
        None,
        SchedulerConfig::from_jobs(Some(1)).expect("scheduler"),
    );
    assert!(
        no_trace
            .failures
            .iter()
            .any(|failure| failure.contains(TRACE_REL))
    );

    super::write_minimal_root(&root, &["agent-queryable-observability"], true);
    fs::remove_file(root.join(MATRIX_REL)).expect("remove matrix");
    let no_matrix = validate(
        &root,
        None,
        SchedulerConfig::from_jobs(Some(1)).expect("scheduler"),
    );
    assert!(
        no_matrix
            .failures
            .iter()
            .any(|failure| failure.contains(MATRIX_REL))
    );

    super::write_minimal_root(&root, &["agent-queryable-observability"], true);
    crate::json_boundary::write_json(&root.join(TRACE_REL), &json!({})).expect("bad trace");
    let bad_trace = validate(
        &root,
        None,
        SchedulerConfig::from_jobs(Some(1)).expect("scheduler"),
    );
    assert!(
        bad_trace
            .failures
            .iter()
            .any(|failure| { failure.contains("foundational_law_trace_entries_missing") })
    );

    super::write_minimal_root(&root, &["agent-queryable-observability"], true);
    let receipt = root.join("absolute-foundational-trace.json");
    let command = FoundationalTraceCommand {
        obligation: Some("agent-queryable-observability".to_string()),
        receipt: receipt.clone(),
        jobs: Some(1),
    };
    let err = run(&root, &command).expect_err("absolute claim receipt rejected");
    assert!(err.contains("root-relative claim artifact path"), "{err}");
    assert!(!receipt.is_file());

    let zero_jobs = FoundationalTraceCommand {
        obligation: Some("agent-queryable-observability".to_string()),
        receipt: PathBuf::from(RECEIPT_REL),
        jobs: Some(0),
    };
    assert!(
        run(&root, &zero_jobs)
            .expect_err("bounded scheduler")
            .contains("scheduler jobs must be at least 1")
    );

    let dispatch = crate::parse_command(&[
        "foundational-trace".to_string(),
        "check".to_string(),
        "--strict".to_string(),
        "--obligation".to_string(),
        "agent-queryable-observability".to_string(),
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
fn foundational_trace_runtime_metrics_report_resource_and_saturation_states() {
    let empty = runtime::from_metrics(&[], 7);
    assert_eq!(empty.cache_mode, "foundational_trace_no_cache");
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
    assert_eq!(mixed.cache_mode, "mixed_foundational_trace_scheduler_modes");
    assert_eq!(
        mixed.resource_measurement_status,
        "mixed_foundational_trace_scheduler_modes"
    );
    assert_eq!(
        mixed.saturation_status,
        "queued_parallel_foundational_trace_checks"
    );
}

#[test]
fn foundational_trace_run_propagates_telemetry_and_receipt_write_errors() {
    let no_manifest = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "foundational-trace-no-manifest",
    );
    super::write_minimal_root(&no_manifest, &["agent-queryable-observability"], true);
    fs::remove_file(no_manifest.join("plugin-manifest-draft.json")).expect("remove manifest");
    let command = FoundationalTraceCommand {
        obligation: Some("agent-queryable-observability".to_string()),
        receipt: PathBuf::from(RECEIPT_REL),
        jobs: Some(1),
    };
    let err = run(&no_manifest, &command).expect_err("candidate digest required");
    assert!(err.contains("plugin-manifest-draft.json"), "{err}");
    fs::remove_dir_all(no_manifest).expect("cleanup no manifest");

    let blocked =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("foundational-trace-blocked");
    super::write_minimal_root(&blocked, &["agent-queryable-observability"], true);
    fs::create_dir_all(blocked.join("validation_artifacts/foundational-trace"))
        .expect("blocked parent root");
    fs::write(
        blocked.join("validation_artifacts/foundational-trace/blocked-parent"),
        "blocked",
    )
    .expect("blocked path");
    let blocked_command = FoundationalTraceCommand {
        receipt: PathBuf::from(
            "validation_artifacts/foundational-trace/blocked-parent/receipt.json",
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
