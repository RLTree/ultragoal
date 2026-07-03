use super::{RuntimeFacts, red, runtime, write_all};
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
    red::write_standalone(&root, &report, RuntimeFacts::from_elapsed_ms(8))
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
    red::write_standalone(&root, &report, RuntimeFacts::from_elapsed_ms(9))
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
fn audit_runtime_telemetry_reports_scheduler_resource_edges() {
    let mixed = runtime::telemetry(
        &json!({
            "scheduler_execution": [
                {
                    "worker_count": 0,
                    "task_count": 1,
                    "queue_depth": 0,
                    "cpu_ms": 5,
                    "memory_bytes": 50,
                    "io_bytes": 500,
                    "cache_mode": "a",
                    "resource_measurement_status": "ra"
                },
                {
                    "worker_count": 2,
                    "task_count": 1,
                    "queue_depth": 1,
                    "cpu_ms": 7,
                    "memory_bytes": 70,
                    "io_bytes": 700,
                    "cache_mode": "b",
                    "resource_measurement_status": "rb"
                }
            ]
        }),
        RuntimeFacts::from_elapsed_ms(9),
    );
    assert_eq!(mixed.cpu_ms, Some(12));
    assert_eq!(mixed.memory_bytes, Some(120));
    assert_eq!(mixed.io_bytes, Some(1200));
    assert_eq!(mixed.cache_mode, "mixed_cache_mode");
    assert_eq!(
        mixed.resource_measurement_status,
        "mixed_resource_measurement_status"
    );
    assert_eq!(mixed.saturation_status, "within_worker_capacity");

    let missing_worker = runtime::telemetry(
        &json!({"scheduler_execution":[{"task_count":1,"queue_depth":0}]}),
        RuntimeFacts::from_elapsed_ms(10),
    );
    assert_eq!(
        missing_worker.saturation_status,
        "scheduler_worker_count_missing"
    );
}
