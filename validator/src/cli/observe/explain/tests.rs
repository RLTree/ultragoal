use super::*;
use serde_json::json;
use std::fs;

#[test]
fn explain_reports_current_failure_and_bad_root_errors() {
    let root = crate::self_tests::boundaries::support::temp_root("observe-explain");
    fs::create_dir_all(&root).expect("root");
    fs::write(root.join("owned.txt"), "owned").expect("owned");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["owned.txt"]}),
    )
    .expect("manifest");
    fs::create_dir_all(root.join("validation_artifacts/ultragoal-audit")).expect("audit dir");
    crate::json_boundary::write_json(
        &root.join("validation_artifacts/ultragoal-audit/validator-receipt.json"),
        &json!({"failures":["final_packet_proof_source_audit_target_digest_mismatch"]}),
    )
    .expect("audit receipt");
    let command = crate::cli::observe::parse(&[
        "observe".to_string(),
        "explain-failure".to_string(),
        "--run-id".to_string(),
        "run-1".to_string(),
    ])
    .expect("parse")
    .expect("observe");
    let receipt = run(&root, &command).expect("explain");
    assert_eq!(
        receipt["explanation"]["known_current_failure"][0],
        "final_packet_proof_source_audit_target_digest_mismatch"
    );
    assert!(
        receipt["why_failed"]
            .as_str()
            .unwrap()
            .contains("final_packet_proof_source_audit_target_digest_mismatch")
    );
    fs::create_dir_all(root.join("validation_artifacts/review")).expect("review dir");
    crate::json_boundary::write_json(
        &root.join("validation_artifacts/review/final-packet-proof.json"),
        &json!({
            "status":"fail",
            "failure":{"observed_failures":["final_packet_proof_registry_ref:plugin_self_law_registry_status_not_pass"]}
        }),
    )
    .expect("packet receipt");
    let packet_receipt = run(&root, &command).expect("explain packet");
    assert_eq!(
        packet_receipt["explanation"]["known_current_failure"][0],
        "final_packet_proof_registry_ref:plugin_self_law_registry_status_not_pass"
    );
    assert!(
        packet_receipt["why_failed"]
            .as_str()
            .unwrap()
            .contains("plugin_self_law_registry_status_not_pass")
    );
    write_run_event(
        &root,
        "run-1",
        "live observability stack is not health-checked",
    );
    let run_receipt = run(&root, &command).expect("explain observed run");
    assert_eq!(
        run_receipt["explanation"]["known_current_failure"][0],
        "live observability stack is not health-checked"
    );
    assert_eq!(run_receipt["observed_run"]["where_failed"], "observe.prove");
    assert_eq!(run_receipt["where_failed"], "observe.prove");
    assert_eq!(run_receipt["next_repair"], "fit every law-bearing command");
    assert_eq!(run_receipt["claim_impact"], "update_goal_blocked");
    fs::remove_file(root.join("validation_artifacts/review/final-packet-proof.json"))
        .expect("remove packet receipt");
    crate::json_boundary::write_json(
        &root.join("validation_artifacts/ultragoal-audit/validator-receipt.json"),
        &json!({
            "failures":[],
            "checks":[
                {"id":"source-audit-pass","status":"pass"},
                {"id":"validator-execution-provenance","status":"fail"}
            ]
        }),
    )
    .expect("audit checks receipt");
    let check_receipt = run(&root, &command).expect("explain checks");
    assert_eq!(
        check_receipt["explanation"]["known_current_failure"][0],
        "live observability stack is not health-checked"
    );
    fs::remove_file(root.join("validation_artifacts/observability/spool/events.jsonl"))
        .expect("remove run telemetry");
    let check_receipt = run(&root, &command).expect("explain checks without telemetry");
    assert_eq!(
        check_receipt["explanation"]["known_current_failure"][0],
        "validator-execution-provenance"
    );
    assert!(
        check_receipt["why_failed"]
            .as_str()
            .unwrap()
            .contains("validator-execution-provenance")
    );
    crate::json_boundary::write_json(
        &root.join("validation_artifacts/ultragoal-audit/validator-receipt.json"),
        &json!({
            "failures":[],
            "checks":{
                "source-audit-pass":{"status":"pass","details":"pass"},
                "validator-execution-provenance":{
                    "status":"fail",
                    "details":"final_packet_proof_source_audit_target_digest_mismatch"
                }
            }
        }),
    )
    .expect("audit object checks receipt");
    let object_check_receipt = run(&root, &command).expect("explain object checks");
    assert!(
        object_check_receipt["why_failed"]
            .as_str()
            .unwrap()
            .contains("final_packet_proof_source_audit_target_digest_mismatch")
    );
    crate::json_boundary::write_json(
        &root.join("validation_artifacts/ultragoal-audit/validator-receipt.json"),
        &json!({
            "failures":[],
            "checks":{
                "source-audit-pass":{"status":"pass","details":"pass"},
                "validator-execution-provenance":{"status":"fail","details":"pass"}
            }
        }),
    )
    .expect("audit object checks without details receipt");
    let object_no_details_receipt =
        run(&root, &command).expect("explain object checks without details");
    assert_eq!(
        object_no_details_receipt["explanation"]["known_current_failure"][0],
        "validator-execution-provenance"
    );
    crate::json_boundary::write_json(
        &root.join("validation_artifacts/ultragoal-audit/validator-receipt.json"),
        &json!({"failures":[],"checks":[{"id":"source-audit-pass","status":"pass"}]}),
    )
    .expect("audit pass receipt");
    let no_failure_receipt = run(&root, &command).expect("explain no failure");
    assert_eq!(
        no_failure_receipt["explanation"]["known_current_failure"][0],
        "no current source-audit failure; inspect final-packet/control receipts"
    );
    fs::remove_file(root.join("validation_artifacts/ultragoal-audit/validator-receipt.json"))
        .expect("remove audit receipt");
    let missing_receipt = run(&root, &command).expect("explain missing audit");
    assert_eq!(
        missing_receipt["explanation"]["known_current_failure"][0],
        "source audit receipt unavailable"
    );
    let bad_root = crate::self_tests::boundaries::support::temp_root("observe-explain-bad");
    fs::create_dir_all(&bad_root).expect("bad root");
    assert!(
        run(&bad_root, &command)
            .unwrap_err()
            .contains("plugin-manifest-draft.json")
    );
    fs::remove_dir_all(root).expect("cleanup explain");
    fs::remove_dir_all(bad_root).expect("cleanup explain bad");
}

fn write_run_event(root: &std::path::Path, run_id: &str, why_failed: &str) {
    let path = root.join("validation_artifacts/observability/spool/events.jsonl");
    fs::create_dir_all(path.parent().unwrap()).expect("spool dir");
    fs::write(
        path,
        format!(
            "{}\n",
            json!({
                "run_id": run_id,
                "status": "fail",
                "why_failed": why_failed,
                "where_failed": "observe.prove",
                "next_repair": "fit every law-bearing command",
                "claim_impact": "update_goal_blocked",
                "law_id": "full-local-observability-stack-integration-non-opaque-failure",
                "check_id": "full-local-observability-stack-integration-non-opaque-failure",
                "claim_id": "gate-92-observability-control-plane",
                "query_hint_logql": "_time:5m operation:observe.prove",
                "query_hint_promql": "ultragoal_command_total{operation=\"observe.prove\"}",
                "query_hint_traceql": "{operation=\"observe.prove\"}"
            })
        ),
    )
    .expect("spool event");
}
