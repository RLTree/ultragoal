use super::{ReceiptFields, RuntimeFacts, emit_receipt, red, write_all};
use serde_json::json;
use std::fs;

const RECEIPT: &str = "validation_artifacts/observability/red-fixture-report.json";

#[test]
fn red_fixture_report_observability_receipt_supports_only_red_report() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("red-report-observe");
    fs::create_dir_all(root.join("validation_artifacts/ultragoal-audit")).expect("audit dir");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":[]}),
    )
    .expect("manifest");
    let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
    let report = root.join("validation_artifacts/ultragoal-audit/red-fixture-report.json");
    crate::json_boundary::write_json(
        &report,
        &json!({
            "status":"pass",
            "target_revision":{"kind":"package_digest","value":candidate},
            "red_fixtures":{"red-one":{"status":"pass"}}
        }),
    )
    .expect("red report");
    red::write_report(&root, &report, None, RuntimeFacts::from_elapsed_ms(5))
        .expect("observability");
    let value = crate::json_boundary::read_json(&root.join(RECEIPT)).expect("red receipt");
    assert_eq!(value["status"], "pass");
    assert_eq!(value["operation"], "red_fixture.report");
    assert_eq!(value["claim_id"], "red_fixture_report");
    assert!(
        value["supported_claims"]
            .as_array()
            .expect("supported")
            .iter()
            .any(|item| item.as_str() == Some("red_fixture_report"))
    );
    assert!(
        value["blocked_claims"]
            .as_array()
            .expect("blocked")
            .iter()
            .any(|item| item.as_str() == Some("update_goal_eligibility"))
    );

    crate::json_boundary::write_json(
        &report,
        &json!({"status":"fail","red_fixtures":{"red-one":{"status":"pass"}}}),
    )
    .expect("empty failure red report");
    red::write_report(&root, &report, None, RuntimeFacts::from_elapsed_ms(6))
        .expect("empty failure observe");
    let empty = crate::json_boundary::read_json(&root.join(RECEIPT)).expect("empty receipt");
    assert_eq!(
        empty["why_failed"],
        "red fixture report missing, malformed, stale, or not pass"
    );

    crate::json_boundary::write_json(
        &report,
        &json!({
            "status":"pass",
            "target_revision":{"kind":"package_digest","value":crate::self_tests::boundaries::workspace_fixtures::sha('f')},
            "red_fixtures":{"red-one":{"status":"pass"}}
        }),
    )
    .expect("stale report");
    red::write_report(&root, &report, None, RuntimeFacts::from_elapsed_ms(7))
        .expect("stale observe");
    let stale = crate::json_boundary::read_json(&root.join(RECEIPT)).expect("stale receipt");
    assert_eq!(stale["status"], "fail");
    assert!(
        stale["why_failed"]
            .as_str()
            .expect("why")
            .contains("does not match current candidate")
    );

    crate::json_boundary::write_json(
        &report,
        &json!({
            "status":"pass",
            "target_package_digest":candidate,
            "red_fixtures":{"red-one":{"status":"pass"}}
        }),
    )
    .expect("fallback report");
    red::write_standalone(&root, &report, RuntimeFacts::from_elapsed_ms(8), &[])
        .expect("fallback observe");
    let fallback = crate::json_boundary::read_json(&root.join(RECEIPT)).expect("fallback receipt");
    assert_eq!(fallback["status"], "pass");

    crate::json_boundary::write_json(
        &report,
        &json!({
            "status":"pass",
            "package_digest":candidate,
            "red_fixtures":{"red-one":{"status":"pass"}}
        }),
    )
    .expect("package digest fallback report");
    red::write_standalone(&root, &report, RuntimeFacts::from_elapsed_ms(9), &[])
        .expect("package digest fallback observe");
    let package_fallback =
        crate::json_boundary::read_json(&root.join(RECEIPT)).expect("package fallback receipt");
    assert_eq!(package_fallback["status"], "pass");
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn red_fixture_report_observability_requires_package_candidate() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("red-report-no-candidate");
    let report = root.join("validation_artifacts/ultragoal-audit/red-fixture-report.json");
    fs::create_dir_all(report.parent().unwrap()).expect("report dir");
    crate::json_boundary::write_json(&report, &json!({"status":"pass"})).expect("report");
    let err = red::write_report(&root, &report, None, RuntimeFacts::from_elapsed_ms(1))
        .expect_err("missing manifest blocks candidate");
    assert!(err.contains("plugin-manifest-draft.json"), "{err}");
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn source_audit_red_report_observability_write_error_is_propagated() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("red-report-write-error");
    fs::create_dir_all(root.join("validation_artifacts/ultragoal-audit")).expect("audit dir");
    fs::create_dir_all(root.join("validation_artifacts/observability/red-fixture-report.json"))
        .expect("receipt path blocker");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":[]}),
    )
    .expect("manifest");
    let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
    let audit = root.join("validation_artifacts/ultragoal-audit/validator-receipt.json");
    crate::json_boundary::write_json(&audit, &json!({"status":"pass","checks":{}}))
        .expect("audit receipt");
    let report = root.join("validation_artifacts/ultragoal-audit/red-fixture-report.json");
    crate::json_boundary::write_json(
        &report,
        &json!({
            "status":"pass",
            "target_revision":{"kind":"package_digest","value":candidate},
            "red_fixtures":{"red-one":{"status":"pass"}}
        }),
    )
    .expect("red report");

    let err = write_all(
        &root,
        &audit,
        Some(&report),
        0,
        false,
        None,
        RuntimeFacts::from_elapsed_ms(2),
    )
    .expect_err("red report receipt write must fail");
    assert!(err.contains("red-fixture-report.json"), "{err}");
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn audit_observability_rejects_absolute_receipt_path() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("audit-absolute-observe");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":[]}),
    )
    .expect("manifest");
    let absolute = root.join("target/audit-observability.json");
    let absolute_text = absolute.to_string_lossy().to_string();
    let err = emit_receipt(
        &root,
        ReceiptFields {
            command: "ultragoal source",
            subcommand: "audit",
            operation: "source.audit",
            surface: "source",
            check_id: "source-audit-observability-binding",
            claim_id: "source_audit",
            artifact_path: "validation_artifacts/ultragoal-audit",
            receipt_path: &absolute_text,
            status: "fail",
            failure_class: "source_audit_check_failure",
            why_failed: "audit failed",
            where_failed: "source.audit",
            next_repair: "rerun source audit after repairing the blocker",
            claim_impact: "source_audit_failed_blocks_claims",
            supported_claims: Vec::new(),
            runtime: crate::cli::observe::telemetry::RuntimeTelemetry {
                duration_ms: 1,
                worker_count: 1,
                task_count: 1,
                queue_depth: 0,
                cpu_ms: None,
                memory_bytes: None,
                io_bytes: None,
                cache_mode: "audit_observability_test".to_string(),
                resource_measurement_status: "test_only".to_string(),
                retry_count: 0,
                backoff_ms: 0,
                saturation_status: "serial_test".to_string(),
                repair_anchor_before: "audit_test_start".to_string(),
                repair_anchor_after: "audit_test_failure".to_string(),
            },
        },
    )
    .expect_err("absolute audit observability receipt rejected");
    assert!(err.contains("root-relative claim artifact path"), "{err}");
    assert!(!absolute.exists());
    fs::remove_dir_all(root).expect("cleanup absolute audit observe");
}
