use serde_json::json;

fn metric(worker_count: usize, task_count: usize, queue_depth: usize) -> crate::scheduler::Metrics {
    crate::scheduler::Metrics {
        task_class: "pure_read_parallel",
        worker_count,
        task_count,
        queue_depth,
        wall_ms: 7,
        cpu_ms: None,
        memory_bytes: None,
        io_bytes: None,
        cache_mode: "no_cache",
        resource_measurement_status: "test_metric",
        deterministic_ordering: true,
        shared_validation_artifact_writes_allowed: false,
    }
}

#[test]
fn typed_boundaries_claim_and_stdout_contracts_are_agent_legible() {
    let failures = vec!["authority-source-binding:raw JSON authority".to_string()];
    assert_eq!(super::claims::why_failed("pass", &failures), "none");
    assert!(super::claims::why_failed("fail", &failures).contains("raw JSON authority"));
    assert_eq!(super::claims::next_repair("pass"), "none");
    assert_eq!(
        super::claims::impact("pass"),
        "supports_typed_boundary_check_source_local_observability_only"
    );
    assert!(super::claims::supported("pass").contains(&"typed_boundary_check".to_string()));
    assert!(super::claims::supported("fail").is_empty());
    assert!(super::claims::blocked().contains(&"update_goal_eligibility".to_string()));

    let value = json!({
        "status": "fail",
        "operation": "typed-boundaries.check",
        "candidate_digest": crate::self_tests::boundaries::workspace_fixtures::sha('b'),
        "receipt_path": "validation_artifacts/test/typed-boundaries.json",
        "run_id": "run-typed",
        "correlation_id": "corr-typed",
        "claim_impact": "typed_boundary_check_failed_blocks_readiness_release_completion_update_goal",
        "supported_claims": [],
        "blocked_claims": ["completion", "update_goal_eligibility"],
        "law_id": "authority-source-binding",
        "check_id": "typed-boundaries-check-observability-binding",
        "why_failed": "typed boundary check failed: raw JSON authority",
        "where_failed": "typed-boundaries.check",
        "next_repair": "repair the named parser boundary"
    });
    let mut value = value;
    value["foundational_law_surface_inventory"] = json!({
        "surface_count": 39,
        "missing_surface_count": 2,
        "package_inventory_missing_count": 1
    });
    let lines = super::stdout::contract(&value);
    assert!(lines[0].contains("ultragoal-typed-boundaries-check fail"));
    assert!(lines[0].contains("authority_surface_count=39"));
    assert!(lines[0].contains("missing_surfaces=2"));
    assert!(lines[0].contains("package_inventory_missing=1"));
    assert!(lines[1].contains("why=typed boundary check failed: raw JSON authority"));
    assert!(lines[1].contains("query_traces='ultragoal observe traces query --run-id run-typed"));
}

#[test]
fn typed_boundaries_runtime_reports_scheduler_saturation() {
    let empty = super::runtime::from_metrics(&[], 5);
    assert_eq!(empty.saturation_status, "no_scheduler_tasks_started");
    let missing_worker = super::runtime::from_metrics(&[metric(0, 1, 1)], 5);
    assert_eq!(
        missing_worker.saturation_status,
        "scheduler_worker_count_missing"
    );
    let queued = super::runtime::from_metrics(&[metric(1, 2, 3)], 5);
    assert_eq!(
        queued.saturation_status,
        "queued_parallel_typed_boundary_checks"
    );
    let normal = super::runtime::from_metrics(&[metric(4, 2, 1)], 5);
    assert_eq!(normal.saturation_status, "within_worker_capacity");
    assert_eq!(normal.cache_mode, "typed_boundary_no_cache");
}
