use super::*;
use serde_json::json;
use std::fs;

mod explanation_fixtures;
mod observation_skip;
mod receipt_fallback;
mod target_edges;
use explanation_fixtures::*;

#[test]
fn explain_reports_current_failure_and_bad_root_errors() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("observe-explain");
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
    let fallback_command =
        crate::cli::observe::parse(&["observe".to_string(), "explain-failure".to_string()])
            .expect("parse")
            .expect("observe");
    let target_command = crate::cli::observe::parse(&[
        "observe".to_string(),
        "explain-failure".to_string(),
        "--run-id".to_string(),
        "run-1".to_string(),
    ])
    .expect("parse")
    .expect("observe");
    let receipt = run(&root, &fallback_command).expect("explain");
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
    let packet_receipt = run(&root, &fallback_command).expect("explain packet");
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
    let missing_target = run(&root, &target_command).expect("explain missing target");
    assert_eq!(missing_target["status"], "fail");
    assert_eq!(
        missing_target["explanation"]["known_current_failure"][0],
        "requested telemetry target unavailable:run_id=run-1"
    );
    assert!(
        missing_target["next_repair"]
            .as_str()
            .unwrap()
            .contains("target command once")
    );
    write_run_event(&root, "run-1", "none");
    let opaque_event_receipt = run(&root, &target_command).expect("explain opaque event");
    assert_eq!(opaque_event_receipt["status"], "fail");
    assert_eq!(
        opaque_event_receipt["explanation"]["known_current_failure"][0],
        "observed telemetry failure is opaque:run-1"
    );
    assert!(
        opaque_event_receipt["next_repair"]
            .as_str()
            .unwrap()
            .contains("failure_class why_failed where_failed")
    );
    write_run_event(
        &root,
        "run-1",
        "live observability stack is not health-checked",
    );
    let run_receipt = run(&root, &target_command).expect("explain observed run");
    assert_eq!(
        run_receipt["explanation"]["known_current_failure"][0],
        "live observability stack is not health-checked"
    );
    assert_eq!(run_receipt["observed_run"]["where_failed"], "observe.prove");
    assert_eq!(run_receipt["where_failed"], "observe.prove");
    assert_eq!(
        run_receipt["next_repair"],
        "reconcile every law-bearing command"
    );
    assert_eq!(
        run_receipt["observed_next_repair"],
        "reconcile every law-bearing command"
    );
    assert_eq!(
        run_receipt["explanation"]["repair_guidance"],
        "reconcile every law-bearing command"
    );
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
    let check_receipt = run(&root, &target_command).expect("explain checks");
    assert_eq!(
        check_receipt["explanation"]["known_current_failure"][0],
        "live observability stack is not health-checked"
    );
    fs::remove_file(root.join("validation_artifacts/observability/spool/events.jsonl"))
        .expect("remove run telemetry");
    write_command_receipt_event(&root, "run-1");
    write_query_receipt_event(&root, "run-1");
    let command_receipt = run(&root, &target_command).expect("explain command receipt event");
    assert_eq!(
        command_receipt["observed_next_repair"],
        "query logs metrics traces then repair the named law"
    );
    assert_eq!(
        command_receipt["explanation"]["known_current_failure"][0],
        "mandatory law validation failed"
    );
    write_failed_metrics_query_receipt(&root, "run-1");
    let failed_metrics_receipt = run(&root, &target_command).expect("explain failed metrics");
    assert_eq!(
        failed_metrics_receipt["explanation"]["query_evidence"]["metrics"]["status"],
        "fail"
    );
    assert!(
        failed_metrics_receipt["explanation"]["query_evidence"]["metrics"]["why_failed"]
            .as_str()
            .unwrap()
            .contains("observability_metric_run_reconciliation_mismatch")
    );
    fs::remove_file(root.join("validation_artifacts/observability/mandatory-law-validation.json"))
        .expect("remove command receipt event");
    let check_receipt = run(&root, &fallback_command).expect("explain checks without telemetry");
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
    let object_check_receipt = run(&root, &fallback_command).expect("explain object checks");
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
        run(&root, &fallback_command).expect("explain object checks without details");
    assert_eq!(
        object_no_details_receipt["explanation"]["known_current_failure"][0],
        "validator-execution-provenance"
    );
    crate::json_boundary::write_json(
        &root.join("validation_artifacts/ultragoal-audit/validator-receipt.json"),
        &json!({"failures":[],"checks":[{"id":"source-audit-pass","status":"pass"}]}),
    )
    .expect("audit pass receipt");
    let no_failure_receipt = run(&root, &fallback_command).expect("explain no failure");
    assert_eq!(
        no_failure_receipt["explanation"]["known_current_failure"][0],
        "no current source-audit failure; inspect final-packet/control receipts"
    );
    fs::remove_file(root.join("validation_artifacts/ultragoal-audit/validator-receipt.json"))
        .expect("remove audit receipt");
    let missing_receipt = run(&root, &fallback_command).expect("explain missing audit");
    assert_eq!(
        missing_receipt["explanation"]["known_current_failure"][0],
        "source audit receipt unavailable"
    );
    let bad_root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("observe-explain-bad");
    fs::create_dir_all(&bad_root).expect("bad root");
    assert!(
        run(&bad_root, &fallback_command)
            .unwrap_err()
            .contains("plugin-manifest-draft.json")
    );
    fs::remove_dir_all(root).expect("cleanup explain");
    fs::remove_dir_all(bad_root).expect("cleanup explain bad");
}
