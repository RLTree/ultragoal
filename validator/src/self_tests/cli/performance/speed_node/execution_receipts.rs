use super::receipts::{digest, executed_speed_node, performance_receipt};
use crate::cli::performance::receipt::same_candidate_pass_failures;
use crate::cli::performance::types::BudgetClass;
use serde_json::json;

#[test]
fn performance_receipt_names_current_speed_node_failure_before_budget_failure() {
    let candidate = digest('a');
    let mut node = executed_speed_node(&candidate);
    node["timing_status"] = json!("fail");
    node["failure_class"] = json!("live_loop_telemetry_reconciliation_missing");
    node["where_failed"] = json!("loop.measure.fmt_check.telemetry_reconciliation");
    node["why_failed"] = json!("metrics query failed boundedness");
    node["next_repair"] = json!("rerun live-loop node after fixing metrics query bounds");
    let mut receipt = performance_receipt(&candidate, node);

    crate::cli::performance::proof::apply_status(&mut receipt, BudgetClass::StrictLocal, 1);

    assert_eq!(receipt["status"], "fail");
    assert_eq!(
        receipt["failure"]["id"],
        "cli_performance_node_speed_proof_failed"
    );
    let observed = receipt["failure"]["observed_value"]
        .as_str()
        .expect("observed value");
    assert!(observed.contains("speed_node=performance_command_roundtrip"));
    assert!(observed.contains("failure_class=live_loop_telemetry_reconciliation_missing"));
    assert!(observed.contains("why_failed=metrics query failed boundedness"));
    assert!(observed.contains("actual_work_duration_ms=10"));
    assert!(observed.contains("cache_hit=false"));
}

#[test]
fn performance_receipt_observed_value_names_missing_and_empty_speed_nodes() {
    let candidate = digest('a');
    let mut missing = performance_receipt(&candidate, executed_speed_node(&candidate));
    missing
        .as_object_mut()
        .expect("receipt")
        .remove("speed_proof");
    crate::cli::performance::proof::apply_status(&mut missing, BudgetClass::StrictLocal, 1);
    assert_eq!(
        missing["failure"]["id"],
        "cli_performance_missing_node_speed_proof"
    );
    assert!(
        missing["failure"]["observed_value"]
            .as_str()
            .expect("missing observed")
            .contains("speed_proof.nodes missing")
    );

    let mut empty = performance_receipt(&candidate, executed_speed_node(&candidate));
    empty["speed_proof"]["nodes"] = json!([]);
    crate::cli::performance::proof::apply_status(&mut empty, BudgetClass::StrictLocal, 1);
    assert_eq!(
        empty["failure"]["id"],
        "cli_performance_missing_node_speed_proof"
    );
    assert!(
        empty["failure"]["observed_value"]
            .as_str()
            .expect("empty observed")
            .contains("speed_proof.nodes=[]")
    );
}

#[test]
fn performance_receipt_observed_value_names_missing_node_fields() {
    let candidate = digest('a');
    let mut receipt = performance_receipt(&candidate, json!({"node_id":"shaped-row"}));
    crate::cli::performance::proof::apply_status(&mut receipt, BudgetClass::StrictLocal, 1);

    assert_eq!(
        receipt["failure"]["id"],
        "cli_performance_node_speed_proof_failed"
    );
    let observed = receipt["failure"]["observed_value"]
        .as_str()
        .expect("observed value");
    assert!(observed.contains("speed_node=shaped-row"));
    assert!(observed.contains("proof_kind=missing"));
    assert!(observed.contains("actual_work_duration_ms=missing"));
    assert!(observed.contains("work_unit_count=missing"));
    assert!(observed.contains("cache_hit=missing"));
}

#[test]
fn speed_node_receipt_rejects_missing_invalid_and_unreconciled_core_fields() {
    let current = digest('a');
    let mut receipt = performance_receipt(&current, executed_speed_node(&current));
    let node = &mut receipt["speed_proof"]["nodes"][0];
    node.as_object_mut().expect("node").remove("proof_kind");
    node["candidate_digest"] = json!("sha256:nothex");
    node["actual_work_duration_ms"] = json!(0);
    node.as_object_mut()
        .expect("node")
        .remove("graph_overhead_ms");
    node["telemetry_reconciliation_status"] = json!("fail");
    let failures = same_candidate_pass_failures(&receipt, &current);
    for expected in [
        "cli_performance_speed_node_missing:performance_command_roundtrip:/proof_kind",
        "cli_performance_speed_node_invalid_digest:performance_command_roundtrip:/candidate_digest",
        "cli_performance_speed_node_timing_proxy_only:performance_command_roundtrip",
        "cli_performance_speed_node_missing:performance_command_roundtrip:/graph_overhead_ms",
        "cli_performance_speed_node_telemetry_not_reconciled:performance_command_roundtrip",
        "cli_performance_speed_node_invalid_proof_kind:performance_command_roundtrip:",
    ] {
        assert!(
            failures.iter().any(|failure| failure == expected),
            "{expected}: {failures:?}"
        );
    }
}

#[test]
fn speed_node_receipt_rejects_failed_timing_status_even_with_core_fields() {
    let current = digest('a');
    let mut receipt = performance_receipt(&current, executed_speed_node(&current));
    let node = &mut receipt["speed_proof"]["nodes"][0];
    node["timing_status"] = json!("fail");
    node["failure_class"] = json!("live_loop_speedup_target_missed");
    let failures = same_candidate_pass_failures(&receipt, &current);
    assert!(
        failures.iter().any(|failure| {
            failure
                == "cli_performance_speed_node_timing_failed:performance_command_roundtrip:live_loop_speedup_target_missed"
        }),
        "{failures:?}"
    );
}

#[test]
fn executed_speed_node_receipt_rejects_cache_and_zero_work_theater() {
    let current = digest('a');
    let mut receipt = performance_receipt(&current, executed_speed_node(&current));
    let node = &mut receipt["speed_proof"]["nodes"][0];
    node["cache_hit"] = json!(true);
    node["work_unit_count"] = json!(0);
    node["command_argv"] = json!([]);
    node.as_object_mut().expect("node").remove("exit_status");
    node.as_object_mut().expect("node").remove("receipt_paths");
    let failures = same_candidate_pass_failures(&receipt, &current);
    for expected in [
        "cli_performance_speed_node_cache_hit_without_verified_proof",
        "cli_performance_speed_node_zero_work_without_cache_equivalence",
        "cli_performance_speed_node_missing:performance_command_roundtrip:/command_argv",
        "cli_performance_speed_node_missing:performance_command_roundtrip:/exit_status",
        "cli_performance_speed_node_missing:performance_command_roundtrip:/receipt_paths",
    ] {
        assert!(
            failures.iter().any(|failure| failure.contains(expected)),
            "{expected}: {failures:?}"
        );
    }
}
