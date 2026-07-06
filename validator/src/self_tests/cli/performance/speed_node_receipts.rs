use crate::cli::performance::receipt::{same_candidate_pass_failures, surface_value_failures};
use crate::cli::performance::types::{BudgetClass, PERFORMANCE_RECEIPT_SCHEMA};
use serde_json::{Value, json};

fn digest(ch: char) -> String {
    crate::self_tests::boundaries::workspace_fixtures::sha(ch)
}

fn strict_budget() -> Value {
    json!({
        "class": "strict_local",
        "cold_p95_ms": 60_000,
        "warm_p95_ms": null,
        "target_ms": 60_000,
        "hard_ceiling_ms": 180_000,
        "threshold_ms": 60_000
    })
}

fn executed_speed_node(candidate: &str) -> Value {
    let node_digest = digest('c');
    json!({
        "node_id": "performance_command_roundtrip",
        "proof_kind": "executed",
        "candidate_digest": candidate,
        "cache_hit": false,
        "command_argv": ["ultragoal", "performance", "prove"],
        "exit_status": 0,
        "work_unit_count": 1,
        "actual_work_duration_ms": 10,
        "graph_overhead_ms": 1,
        "result_digest": node_digest,
        "output_digest": node_digest,
        "receipt_paths": ["validation_artifacts/performance/performance.json"],
        "telemetry_reconciliation_status": "pass",
        "claim_impact": "supports_performance_command_speed_only"
    })
}

fn verified_cache_speed_node(candidate: &str) -> Value {
    let mut node = executed_speed_node(candidate);
    let node_digest = digest('c');
    node["proof_kind"] = json!("verified_cache_hit");
    node["cache_hit"] = json!(true);
    node["work_unit_count"] = json!(0);
    node["cache_key"] = json!(digest('d'));
    node["current_input_digest"] = json!(digest('e'));
    node["validator_version"] = json!("validator-v1");
    node["law_version"] = json!("law-v1");
    node["schema_version"] = json!("schema-v1");
    node["fixture_version"] = json!("fixture-v1");
    node["prior_result_digest"] = json!(node_digest);
    node["replayed_output_digest"] = json!(node_digest);
    node["equivalence_status"] = json!("verified_same_candidate_cache_replay");
    node["invalidation_proof"] = json!("current input, law, schema, fixture, and cache keys match");
    node
}

fn performance_receipt(candidate: &str, node: Value) -> Value {
    json!({
        "schema": PERFORMANCE_RECEIPT_SCHEMA,
        "status": "pass",
        "claim_ceiling": "performance_proven",
        "command": {
            "name": "performance_prove",
            "argv": ["ultragoal", "performance", "prove"]
        },
        "budget": strict_budget(),
        "digests": {"candidate": candidate},
        "cache": {"mode": "disabled", "no_cache_mode_result": "executed_without_cache"},
        "concurrency": {
            "worker_count": 1,
            "queue_depth": 0,
            "isolation_namespace": "typed_serial_reason"
        },
        "telemetry": {
            "wall_clock_ms": 1,
            "cpu_ms": null,
            "peak_memory_bytes": null,
            "io_bytes": null
        },
        "performance_regression": {"status": "pass"},
        "speed_proof": {"nodes": [node]},
        "failure": null,
        "blocked_claim_classes": [],
        "supported_claim_classes": ["routine_usability"]
    })
}

#[test]
fn performance_receipt_pass_requires_complete_executed_node_evidence() {
    let candidate = digest('a');
    let mut receipt = performance_receipt(&candidate, executed_speed_node(&candidate));
    crate::cli::performance::proof::apply_status(&mut receipt, BudgetClass::StrictLocal, 1);
    assert_eq!(receipt["status"], "pass");
    assert_eq!(receipt["claim_ceiling"], "performance_proven");
    assert_eq!(receipt["performance_regression"]["status"], "pass");
    assert_eq!(receipt["exit_code"], 0);
    assert_eq!(
        receipt["supported_claim_classes"],
        json!(["routine_usability"])
    );
    assert_eq!(receipt["blocked_claim_classes"], json!([]));
    assert_eq!(receipt["failure"], Value::Null);
    assert!(surface_value_failures(&receipt).is_empty());
}

#[test]
fn performance_receipt_names_budget_failure_only_after_speed_proof_is_real() {
    let candidate = digest('a');
    let mut receipt = performance_receipt(&candidate, executed_speed_node(&candidate));
    crate::cli::performance::proof::apply_status(&mut receipt, BudgetClass::StrictLocal, 60_001);
    assert_eq!(receipt["status"], "fail");
    assert_eq!(receipt["failure"]["id"], "cli_performance_budget_exceeded");
    assert_eq!(receipt["failure"]["observed_value"], "wall_clock_ms=60001");
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

#[test]
fn verified_cache_speed_node_receipt_requires_same_candidate_equivalence() {
    let current = digest('a');
    let mut receipt = performance_receipt(&current, verified_cache_speed_node(&current));
    assert!(same_candidate_pass_failures(&receipt, &current).is_empty());
    let node = &mut receipt["speed_proof"]["nodes"][0];
    node["cache_hit"] = json!(false);
    node["work_unit_count"] = json!(1);
    node["cache_key"] = json!("sha256:nothex");
    node["prior_result_digest"] = json!(digest('b'));
    node["replayed_output_digest"] = json!(digest('d'));
    node["equivalence_status"] = json!("not_verified");
    node["invalidation_proof"] = json!("");
    let failures = same_candidate_pass_failures(&receipt, &current);
    for expected in [
        "cli_performance_speed_node_cache_hit_missing",
        "cli_performance_speed_node_cache_replay_has_work_units",
        "cli_performance_speed_node_invalid_digest:performance_command_roundtrip:/cache_key",
        "cli_performance_speed_node_missing:performance_command_roundtrip:/invalidation_proof",
        "cli_performance_speed_node_unverified_cache_reuse",
    ] {
        assert!(
            failures.iter().any(|failure| failure.contains(expected)),
            "{expected}: {failures:?}"
        );
    }
}

#[test]
fn verified_cache_speed_node_receipt_reports_each_replay_mismatch_kind() {
    let current = digest('a');
    for (field, value, expected) in [
        (
            "prior_result_digest",
            digest('b'),
            "cli_performance_speed_node_unverified_cache_reuse",
        ),
        (
            "replayed_output_digest",
            digest('d'),
            "cli_performance_speed_node_unverified_cache_reuse",
        ),
        (
            "equivalence_status",
            "not_verified".to_string(),
            "cli_performance_speed_node_unverified_cache_reuse",
        ),
        (
            "result_digest",
            Value::Null.to_string(),
            "cli_performance_speed_node_missing:performance_command_roundtrip:/result_digest",
        ),
    ] {
        let mut receipt = performance_receipt(&current, verified_cache_speed_node(&current));
        let node = &mut receipt["speed_proof"]["nodes"][0];
        if field == "result_digest" {
            node.as_object_mut().expect("node").remove(field);
        } else {
            node[field] = json!(value);
        }
        let failures = same_candidate_pass_failures(&receipt, &current);
        assert!(
            failures.iter().any(|failure| failure.contains(expected)),
            "{expected}: {failures:?}"
        );
    }
}
