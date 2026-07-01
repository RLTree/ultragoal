use serde_json::{Value, json};
use std::path::Path;

pub(super) fn attach(root: &Path, receipt: &mut Value, status: &str, why: &str) {
    let obs = crate::cli::observe::telemetry::command_receipt(
        root,
        crate::cli::observe::telemetry::CommandTelemetry {
            command: "ultragoal final-packet",
            subcommand: "prove",
            operation: "final-packet.prove",
            surface: "source",
            law_id: "final-packet-proof",
            check_id: if status == "pass" {
                "none"
            } else {
                "unsupported_live_surface"
            },
            claim_id: "final_packet_correctness",
            artifact_path: "validation_artifacts/review",
            receipt_path: "validation_artifacts/review/final-packet-proof.json",
            status,
            failure_class: if status == "pass" {
                "none"
            } else {
                "unsupported_live_surface"
            },
            why_failed: why,
            where_failed: if status == "pass" {
                "none"
            } else {
                "validation_artifacts/review/final-packet-proof.json#/failure/observed_failures/0"
            },
            next_repair: if status == "pass" {
                "none"
            } else {
                "produce live same-surface registry proof or keep registry-dependent claims blocked"
            },
            claim_impact: if status == "pass" {
                "final_packet_evidence_dereferenced_only"
            } else {
                "final_packet_correctness_review_readiness_release_completion_update_goal_blocked"
            },
            blocked_claims: blocked_claims(receipt),
            supported_claims: if status == "pass" {
                vec!["final_packet_evidence_dereferenced".to_string()]
            } else {
                vec![]
            },
            runtime: None,
            emit: false,
        },
    )
    .expect("self-test observability receipt");
    receipt["candidate_digest"] = obs["candidate_digest"].clone();
    receipt["run_id"] = obs["run_id"].clone();
    receipt["correlation_id"] = obs["correlation_id"].clone();
    receipt["proof_law_id"] = json!("final-packet-proof");
    receipt["proof_check_id"] = json!(if status == "pass" {
        "none"
    } else {
        "unsupported_live_surface"
    });
    receipt["proof_claim_id"] = json!("final_packet_correctness");
    receipt["why_failed"] = json!(why);
    receipt["where_failed"] = obs["where_failed"].clone();
    receipt["next_repair"] = obs["next_repair"].clone();
    receipt["claim_impact"] = obs["event"]["claim_impact"].clone();
    receipt["observability"] = obs;
}

fn blocked_claims(receipt: &Value) -> Vec<String> {
    receipt
        .get("blocked_claim_classes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(ToOwned::to_owned)
        .collect()
}
