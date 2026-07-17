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
        artifacts_are_isolated: true,
    }
}

#[test]
fn package_inventory_maps_boundary_failures_to_product_checks() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "package-inventory-boundaries",
    );
    let missing_manifest = super::inventory_task(&root, super::inventory_closure_failures)();
    assert!(
        missing_manifest
            .iter()
            .any(|failure| failure.contains("plugin-manifest-draft.json load failed")),
        "{missing_manifest:?}"
    );

    std::fs::create_dir_all(root.join("skills/demo")).expect("skill dir");
    std::fs::write(root.join("root-ref.md"), "root").expect("root ref");
    std::fs::write(root.join("skills/demo/SKILL.md"), "[root](root-ref.md)").expect("skill");
    let skill_failures =
        super::skill_link_failures(&root, &json!({"skills":[{"path":"skills/demo/SKILL.md"}]}));
    assert!(
        skill_failures
            .iter()
            .any(|failure| failure
                .starts_with("skill-inventory-closure:skill_local_reference_missing")),
        "{skill_failures:?}"
    );

    let resource_failures = super::resource_purpose_failures(
        &root,
        &json!({"resources":["artifacts/stale-proof.json"]}),
    );
    assert!(
        resource_failures
            .iter()
            .any(|failure| failure.contains("stale_artifact_resource_packaged")),
        "{resource_failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn package_inventory_claim_and_stdout_contracts_are_agent_legible() {
    let failures = vec!["plugin-inventory-closure:missing resource".to_string()];
    assert_eq!(super::claim_ceiling::why_failed("pass", &failures), "none");
    assert!(super::claim_ceiling::why_failed("fail", &failures).contains("missing resource"));
    assert_eq!(super::claim_ceiling::next_repair("pass"), "none");
    assert_eq!(
        super::claim_ceiling::impact("pass"),
        "supports_package_inventory_source_local_observability_only"
    );
    assert!(
        super::claim_ceiling::supported("pass")
            .contains(&"package_inventory_source_local".to_string())
    );
    assert!(super::claim_ceiling::supported("fail").is_empty());
    assert!(super::claim_ceiling::blocked().contains(&"update_goal_eligibility".to_string()));

    let value = json!({
        "status": "fail",
        "operation": "package.inventory",
        "candidate_digest": crate::self_tests::boundaries::workspace_fixtures::sha('a'),
        "receipt_path": "validation_artifacts/package/package-inventory.json",
        "run_id": "run-package",
        "correlation_id": "corr-package",
        "claim_impact": "package_inventory_failed_blocks_package_readiness_release_completion_update_goal",
        "supported_claims": [],
        "blocked_claims": ["completion", "update_goal_eligibility"],
        "law_id": "cli-control-plane-authority",
        "check_id": "plugin-inventory-closure",
        "why_failed": "package inventory check failed: missing resource",
        "where_failed": "package.inventory",
        "next_repair": "repair the named package resource"
    });
    let lines = super::stdout::contract(&value, &failures);
    assert!(lines[0].contains("ultragoal-package-inventory fail"));
    assert!(lines[1].contains("first_failure=plugin-inventory-closure:missing resource"));
    assert!(lines[1].contains("query_logs='ultragoal observe logs query --run-id run-package"));
}

#[test]
fn package_inventory_runtime_reports_scheduler_saturation() {
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
        "queued_parallel_package_inventory_checks"
    );
    let normal = super::runtime::from_metrics(&[metric(4, 2, 1)], 5);
    assert_eq!(normal.saturation_status, "within_worker_capacity");
    assert_eq!(normal.cache_mode, "package_inventory_no_cache");
}
