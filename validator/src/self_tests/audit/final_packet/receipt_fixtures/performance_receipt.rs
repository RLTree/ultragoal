use serde_json::{Value, json};
use std::path::Path;

pub(super) fn ref_for(root: &Path, candidate: &str) -> Value {
    super::ref_for(
        root,
        "validation_artifacts/cli/performance-receipt.json",
        &json!({
            "schema":"harness-ultragoal.cli-performance-receipt.v1",
            "status":"pass",
            "claim_ceiling":"source_local_speed_node_timing_only",
            "command":{"argv":["ultragoal","performance","prove"]},
            "budget":{
                "class":"strict_local",
                "cold_p95_ms":60000,
                "warm_p95_ms":null,
                "target_ms":60000,
                "hard_ceiling_ms":180000,
                "threshold_ms":60000
            },
            "digests":{"candidate":candidate},
            "cache":{"mode":"disabled","no_cache_mode_result":"executed_without_cache"},
            "concurrency":{"worker_count":1,"queue_depth":0,"isolation_namespace":"test_isolated_no_shared_artifact_writes"},
            "telemetry":{"wall_clock_ms":1,"cpu_ms":null,"peak_memory_bytes":null,"io_bytes":null},
            "performance_regression":{"status":"pass"},
            "speed_proof":{"nodes":[node_speed_evidence(candidate)]},
            "failure":null,
            "blocked_claim_classes":[],
            "supported_claim_classes":["routine_usability"]
        }),
    )
}

fn node_speed_evidence(candidate: &str) -> Value {
    let digest = crate::self_tests::boundaries::workspace_fixtures::sha('c');
    json!({
        "node_id": "performance_prove",
        "proof_kind": "executed",
        "timing_status": "pass",
        "failure_class": "none",
        "where_failed": "none",
        "why_failed": "none",
        "next_repair": "none",
        "candidate_digest": candidate,
        "cache_hit": false,
        "command_argv": ["ultragoal", "performance", "prove"],
        "exit_status": 0,
        "work_unit_count": 1,
        "actual_work_duration_ms": 1,
        "graph_overhead_ms": 1,
        "result_digest": digest,
        "output_digest": digest,
        "receipt_paths": ["validation_artifacts/cli/performance-receipt.json"],
        "telemetry_reconciliation_status": "pass",
        "claim_name": "source-local speed node timing claim",
        "product_behavior_observed": "real ultragoal performance prove command execution",
        "proof_surface": "speed node receipt with command argv, exit status, result digest, output digest, and telemetry reconciliation",
        "independent_reconciliation_surface": "same-candidate logs, metrics, traces, explain output, and performance receipt",
        "claim_impact": "supports_routine_usability_only"
    })
}
