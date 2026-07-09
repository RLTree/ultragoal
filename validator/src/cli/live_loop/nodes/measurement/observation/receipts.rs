use super::*;
use serde_json::json;
use std::path::{Path, PathBuf};

pub(super) fn measure_command() -> LiveLoopCommand {
    LiveLoopCommand {
        action: crate::cli::live_loop::LiveLoopAction::Measure,
        tier: "hot".to_string(),
        cache_mode: "verified-local".to_string(),
        jobs: None,
        receipt: PathBuf::from("validation_artifacts/observability/live-loop-node-timing.json"),
        node_id: Some("changed_files".to_string()),
        measure_all: false,
    }
}

pub(super) fn command_run(duration_ms: u64) -> FullCommandRun {
    FullCommandRun {
        exit_code: 0,
        status_success: true,
        launch_error: false,
        duration_ms,
        stdout_digest: "sha256:stdout".to_string(),
        stderr_digest: "sha256:stderr".to_string(),
        executed_test_count: None,
        failure: Default::default(),
    }
}

pub(super) fn observe_receipt(
    roundtrip: query::RoundtripQuery,
    status: &str,
    run_id: &str,
    correlation_id: &str,
) -> query::ObserveReceipt {
    query::ObserveReceipt {
        receipt: format!(
            "validation_artifacts/observability/live-loop/commands/{roundtrip:?}.json"
        ),
        exit_code: 0,
        status: status.to_string(),
        duration_ms: 7,
        value: match roundtrip {
            query::RoundtripQuery::ExplainFailure => json!({
                "status": status,
                "roundtrip": format!("{roundtrip:?}"),
                "run_id": run_id,
                "correlation_id": correlation_id,
                "bounded_output_proof": "pass",
                "failure_class": "none",
                "claim_impact": "observability_evidence_only",
                "explanation": {
                    "query_evidence": {
                        "logs": {"status":"pass"},
                        "metrics": {"status":"pass"},
                        "traces": {"status":"pass"}
                    }
                }
            }),
            _ => json!({
                "status": status,
                "roundtrip": format!("{roundtrip:?}"),
                "run_id": run_id,
                "correlation_id": correlation_id,
                "bounded_output_status": "pass",
                "redaction_status": "pass",
                "row_count": 1,
                "result_digest": "sha256:1111111111111111111111111111111111111111111111111111111111111111"
            }),
        },
    }
}

pub(super) fn write_minimal_manifest(root: &Path) -> String {
    std::fs::create_dir_all(root).expect("root");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["plugin-manifest-draft.json"]}),
    )
    .expect("manifest");
    crate::package::inventory::package_digest(root).expect("candidate")
}
