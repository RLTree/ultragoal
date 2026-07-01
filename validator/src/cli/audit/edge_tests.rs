use super::*;
use serde_json::json;
use std::fs;

#[test]
fn source_audit_run_reports_observability_write_failure() {
    let root = crate::self_tests::boundaries::support::temp_root("source-audit-run-observe-error");
    fs::create_dir_all(&root).expect("root");
    fs::write(root.join("owned.txt"), "owned").expect("owned");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["owned.txt"]}),
    )
    .expect("manifest");
    write_minimal_law_surfaces(&root);
    fs::create_dir_all(root.join("validation_artifacts")).expect("artifacts");
    fs::write(root.join("validation_artifacts/observability"), "not a dir")
        .expect("observability blocker");
    let receipt = root.join("validation_artifacts/ultragoal-audit/validator-receipt.json");
    let err = run(RunArgs {
        root: root.clone(),
        receipt,
        red_report: None,
        target_repo: None,
        mode: "source".to_string(),
        require_observability: false,
        require_product_cohesion: false,
        jobs: None,
    })
    .expect_err("observability write must fail");
    assert!(err.contains("validation_artifacts/observability"));
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn audit_observability_reports_empty_failure_and_emit_errors() {
    let root = crate::self_tests::boundaries::support::temp_root("audit-observe-edges");
    fs::create_dir_all(&root).expect("root");
    fs::write(root.join("owned.txt"), "owned").expect("owned");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["owned.txt"]}),
    )
    .expect("manifest");
    let receipt = root.join("validation_artifacts/ultragoal-audit/validator-receipt.json");
    crate::json_boundary::write_json(&receipt, &json!({"status":"pass","checks":{}}))
        .expect("receipt");
    observability::write_all(
        &root,
        &receipt,
        None,
        1,
        false,
        None,
        observability::RuntimeFacts::from_elapsed_ms(3),
    )
    .expect("observe");
    let value =
        crate::json_boundary::read_json(&root.join(observability::SOURCE_RECEIPT)).expect("source");
    assert_eq!(
        value["why_failed"],
        "source audit failed without check details"
    );

    let bad_root = crate::self_tests::boundaries::support::temp_root("audit-observe-no-root");
    let err = observability::emit_receipt(
        &bad_root,
        observability::ReceiptFields {
            command: "ultragoal source",
            subcommand: "audit",
            operation: "source.audit",
            surface: "source",
            check_id: "source-audit-observability-binding",
            claim_id: "source_audit",
            artifact_path: "validation_artifacts/ultragoal-audit",
            receipt_path: observability::SOURCE_RECEIPT,
            status: "fail",
            failure_class: "source_audit_check_failure",
            why_failed: "missing package manifest",
            where_failed: "source.audit",
            next_repair: "create package manifest",
            claim_impact: "source_audit_failed_blocks_claims",
            supported_claims: Vec::new(),
            runtime: crate::cli::observe::telemetry::RuntimeTelemetry {
                duration_ms: 3,
                worker_count: 0,
                task_count: 0,
                queue_depth: 0,
                cpu_ms: None,
                memory_bytes: None,
                io_bytes: None,
                cache_mode: "pre_scheduler_guard_no_cache".to_string(),
                resource_measurement_status: "pre_scheduler_guard_no_scheduler_metrics".to_string(),
                retry_count: 0,
                backoff_ms: 0,
                saturation_status: "no_scheduler_tasks_started".to_string(),
                repair_anchor_before: "source_audit_command_start".to_string(),
                repair_anchor_after: "source_audit_observability_emit".to_string(),
            },
        },
    )
    .expect_err("missing package manifest blocks telemetry receipt");
    assert!(err.contains("plugin-manifest-draft.json"));

    emit_parse_error_observability(
        &root,
        &[
            "audit".to_string(),
            "--mode".to_string(),
            "bogus".to_string(),
        ],
        "invalid audit --mode: bogus",
        4,
    )
    .expect("bare audit parse error observation");
    let bare =
        crate::json_boundary::read_json(&root.join(observability::SOURCE_RECEIPT)).expect("bare");
    assert_eq!(bare["operation"], "source.audit");
    assert_eq!(bare["status"], "fail");
    assert_eq!(bare["event"]["cache_mode"], "parser_rejection_no_cache");
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn source_audit_stdout_contract_reports_pass_and_fail_claim_ceiling() {
    let root = crate::self_tests::boundaries::support::temp_root("source-audit-stdout-contract");
    fs::create_dir_all(&root).expect("root");
    fs::write(root.join("owned.txt"), "owned").expect("owned");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["owned.txt"]}),
    )
    .expect("manifest");
    write_minimal_law_surfaces(&root);
    let receipt = root.join("validation_artifacts/ultragoal-audit/validator-receipt.json");
    fs::create_dir_all(receipt.parent().unwrap()).expect("audit dir");
    crate::json_boundary::write_json(
        &receipt,
        &json!({
            "status":"fail",
            "checks": {
                "coverage-receipt": {"status":"fail"},
                "schema-valid": {"status":"pass"}
            }
        }),
    )
    .expect("audit receipt");

    observability::write_all(
        &root,
        &receipt,
        None,
        1,
        false,
        None,
        observability::RuntimeFacts::from_elapsed_ms(4),
    )
    .expect("fail observe");
    let fail =
        crate::json_boundary::read_json(&root.join(observability::SOURCE_RECEIPT)).expect("fail");
    let fail_lines = observability::stdout_contract_for_test(&fail);
    assert_eq!(fail_lines.len(), 2);
    assert!(fail_lines[0].contains("ultragoal-audit-observe fail"));
    assert!(fail_lines[0].contains("operation=source.audit"));
    assert!(fail_lines[0].contains("candidate=sha256:"));
    assert!(fail_lines[0].contains("supported_claims=none"));
    assert!(fail_lines[0].contains(
        "unsupported_claims=completion,readiness,release,reviewer_exposure,app_registry_exposure,final_packet_correctness,update_goal_eligibility"
    ));
    assert!(
        fail_lines[1]
            .contains("failed_law=full-local-observability-stack-integration-non-opaque-failure")
    );
    assert!(fail_lines[1].contains("failed_check=source-audit-observability-binding"));
    assert!(fail_lines[1].contains("why=source audit failed checks: total_failures=1"));
    assert!(fail_lines[1].contains("first_check=coverage-receipt"));
    assert!(fail_lines[1].contains("root_group=source_audit_check_failure"));
    assert!(fail_lines[1].contains("where=source.audit"));
    let run_id = fail["run_id"].as_str().expect("run id");
    assert!(fail_lines[1].contains(&format!(
        "query_logs='ultragoal observe logs query --run-id {run_id} --limit 100'"
    )));
    assert!(fail_lines[1].contains(
        "query_metrics='ultragoal observe metrics query --query 'sum by (__name__,operation,status,check_id,claim_id,surface,failure_class,exporter,saturation_status) (max_over_time({__name__=~\"ultragoal_command_total|ultragoal_command_duration_ms|ultragoal_command_task_count|ultragoal_command_queue_depth\",operation=\"source.audit\""
    ));
    let stale_metric_hint = format!(
        "{}{}{}",
        "query_metrics='ultragoal observe metrics query --", "run-id ", run_id
    );
    assert!(!fail_lines[1].contains(&stale_metric_hint));
    assert!(fail_lines[1].contains(&format!(
        "query_traces='ultragoal observe traces query --run-id {run_id} --limit 100'"
    )));

    crate::json_boundary::write_json(
        &receipt,
        &json!({"status":"pass","checks":{"schema-valid":{"status":"pass"}}}),
    )
    .expect("pass receipt");
    observability::write_all(
        &root,
        &receipt,
        None,
        0,
        false,
        None,
        observability::RuntimeFacts::from_elapsed_ms(5),
    )
    .expect("pass observe");
    let pass =
        crate::json_boundary::read_json(&root.join(observability::SOURCE_RECEIPT)).expect("pass");
    let pass_lines = observability::stdout_contract_for_test(&pass);
    assert_eq!(pass_lines.len(), 1);
    assert!(pass_lines[0].contains("ultragoal-audit-observe pass"));
    assert!(pass_lines[0].contains("operation=source.audit"));
    assert!(
        pass_lines[0].contains("supported_claims=source_local_audit_checks,red_fixture_report")
    );
    assert!(pass_lines[0].contains(
        "unsupported_claims=completion,readiness,release,reviewer_exposure,app_registry_exposure,final_packet_correctness,update_goal_eligibility"
    ));
    fs::remove_dir_all(root).expect("cleanup");
}

fn write_minimal_law_surfaces(root: &std::path::Path) {
    fs::create_dir_all(root.join("examples/generated")).expect("generated dir");
    crate::json_boundary::write_json(&root.join("schemas/schema-catalog.json"), &json!([]))
        .expect("schema catalog");
    crate::json_boundary::write_json(
        &root.join("docs/mandatory-law-surfaces.json"),
        &json!({"laws":[]}),
    )
    .expect("mandatory laws");
    crate::json_boundary::write_json(
        &root.join("docs/source-obligation-matrix.json"),
        &json!({"obligations":[]}),
    )
    .expect("obligations");
    crate::json_boundary::write_json(
        &root.join("templates/agent-standards/enforcement.json"),
        &json!({"rows":[]}),
    )
    .expect("standards");
    crate::json_boundary::write_json(&root.join("templates/RED_FIXTURES.json"), &json!([]))
        .expect("red fixtures");
}
