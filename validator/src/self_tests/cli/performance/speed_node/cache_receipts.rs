use super::receipts::{digest, performance_receipt, verified_cache_speed_node};
use crate::cli::performance::receipt::same_candidate_pass_failures;
use serde_json::{Value, json};

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
