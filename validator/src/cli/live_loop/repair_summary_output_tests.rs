use super::{LiveLoopAction, LiveLoopCommand, summary_line};
use serde_json::json;

#[test]
fn loop_repair_summary_output_names_trace_and_query_routes() {
    let command = LiveLoopCommand {
        action: LiveLoopAction::Run,
        tier: "hot".to_string(),
        cache_mode: "verified-local".to_string(),
        jobs: None,
        receipt: "validation_artifacts/observability/loop-run.json".into(),
        node_id: None,
        measure_all: false,
    };
    let receipt = json!({
        "status": "fail",
        "candidate_digest": "sha256:current",
        "duration_ms": 42,
        "worker_count": 3,
        "task_count": 7,
        "queue_depth": 7,
        "critical_path": "current_digest -> AuditContext",
        "claim_ceiling": "source-local loop proof only",
        "observability": {
            "run_id": "run-loop",
            "correlation_id": "corr-loop",
            "trace_id": "trace-loop",
            "trace": {"span_id": "span-loop"},
            "query_examples": [
                "ultragoal observe logs query --run-id run-loop --limit 100",
                "ultragoal observe metrics query --run-id run-loop --limit 100",
                "ultragoal observe traces query --run-id run-loop --limit 100"
            ]
        }
    });
    let blocker = json!({
        "id": "fmt_check",
        "why_failed": "missing timing proof",
        "next_repair": "measure fmt",
        "narrow_rerun": "cargo fmt --all --check",
        "broad_rerun": "source audit once after narrow observable proof passes"
    });
    let line = summary_line(&command, &receipt, &blocker);
    for expected in [
        "trace_id=trace-loop",
        "span_id=span-loop",
        "query_logs='ultragoal observe logs query --run-id run-loop --limit 100'",
        "query_metrics='ultragoal observe metrics query --run-id run-loop --limit 100'",
        "query_traces='ultragoal observe traces query --run-id run-loop --limit 100'",
    ] {
        assert!(line.contains(expected), "{line}");
    }
}
