use serde_json::json;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_TEMP_ROOT: AtomicU64 = AtomicU64::new(0);

pub(super) fn digest(ch: char) -> String {
    crate::self_tests::boundaries::workspace_fixtures::sha(ch)
}

pub(super) fn temp_root() -> std::path::PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let sequence = NEXT_TEMP_ROOT.fetch_add(1, Ordering::SeqCst);
    std::env::current_dir()
        .expect("cwd")
        .join("target")
        .join(format!(
            "ultragoal-performance-speed-nodes-{}-{stamp}-{sequence}",
            std::process::id()
        ))
}

pub(super) fn verified_cache_row(candidate: &str) -> serde_json::Value {
    let result = digest('c');
    let output = digest('d');
    json!({
        "node_id": "fmt_check",
        "proof_kind": "verified_cache_hit",
        "candidate_digest": candidate,
        "timing_status": "pass",
        "failure_class": "none",
        "where_failed": "none",
        "why_failed": "none",
        "next_repair": "none",
        "cache_hit": true,
        "work_unit_count": 0,
        "actual_work_duration_ms": 1,
        "graph_overhead_ms": 1,
        "result_digest": result,
        "output_digest": output,
        "telemetry_reconciliation_status": "pass",
        "claim_impact": "supports_live_loop_node_timing_only_no_readiness_release_completion_update_goal",
        "verified_local_command_argv": ["bash", "-lc", "cargo fmt --all --check"],
        "command_argv": ["bash", "-lc", "cargo fmt --all --check"],
        "verified_local_exit_code": 0,
        "exit_status": 0,
        "receipt_paths": ["validation_artifacts/observability/live-loop-node-timing.json"],
        "artifact_paths": ["validation_artifacts/observability/live-loop-node-timing.json"],
        "receipt_path": "validation_artifacts/observability/live-loop-node-timing.json",
        "verified_local_failure": {
            "receipt": "validation_artifacts/observability/live-loop/commands/fmt_check-command-observation.json"
        },
        "cache_key": digest('e'),
        "current_input_digest": digest('f'),
        "validator_version": "ultragoal-rust",
        "law_version": "observability-live-loop",
        "schema_version": "harness-ultragoal.live-loop-node-timing.v1",
        "fixture_version": "source-tree-current",
        "prior_result_digest": result,
        "replayed_output_digest": output,
        "cache_equivalence_status": "pass",
        "equivalence_status": "verified_same_candidate_cache_replay",
        "invalidation_proof": "cache_key_current_input_digest_command_versions_and_candidate_row_matched"
    })
}

pub(super) fn executed_row(candidate: &str) -> serde_json::Value {
    json!({
        "node_id": "fmt_check",
        "proof_kind": "executed",
        "candidate_digest": candidate,
        "timing_status": "pass",
        "failure_class": "none",
        "where_failed": "none",
        "why_failed": "none",
        "next_repair": "none",
        "cache_hit": false,
        "work_unit_count": 3,
        "actual_work_duration_ms": 1253,
        "graph_overhead_ms": 1,
        "result_digest": digest('c'),
        "output_digest": digest('d'),
        "telemetry_reconciliation_status": "pass",
        "claim_impact": "supports_live_loop_node_timing_only_no_readiness_release_completion_update_goal",
        "verified_local_command_argv": ["bash", "-lc", "cargo fmt --all --check"],
        "command_argv": ["bash", "-lc", "cargo fmt --all --check"],
        "verified_local_exit_code": 0,
        "exit_status": 0,
        "receipt_paths": ["validation_artifacts/observability/live-loop-node-timing.json"],
        "artifact_paths": ["validation_artifacts/observability/live-loop-node-timing.json"],
        "receipt_path": "validation_artifacts/observability/live-loop-node-timing.json",
        "verified_local_failure": {
            "receipt": "validation_artifacts/observability/live-loop/commands/fmt_check-command-observation.json"
        }
    })
}

pub(super) fn write_timing_fixture(root: &std::path::Path, value: serde_json::Value) {
    std::fs::write(
        root.join(super::super::NODE_TIMING_REL),
        serde_json::to_string_pretty(&value).expect("json"),
    )
    .expect("timing");
}
