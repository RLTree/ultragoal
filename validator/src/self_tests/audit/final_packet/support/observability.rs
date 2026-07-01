use serde_json::{Value, json};
use std::path::Path;

pub(super) fn attach(root: &Path, receipt: &mut Value, status: &str, why: &str) {
    let mut obs = crate::cli::observe::telemetry::command_receipt(
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
    attach_spans(receipt, &mut obs);
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

fn attach_spans(receipt: &Value, obs: &mut Value) {
    let event = obs["event"].clone();
    let Some(children) = obs
        .pointer_mut("/trace/child_spans")
        .and_then(Value::as_array_mut)
    else {
        return;
    };
    let root_span = event
        .get("span_id")
        .and_then(Value::as_str)
        .unwrap_or("span-root");
    for (label, path, status, digest) in dereferenced_receipts(receipt) {
        let mut span = event.clone();
        span["schema"] = json!("harness-ultragoal.observability-trace.v1");
        span["span_id"] = json!(child_span_id(root_span, &label));
        span["parent_span_id"] = json!(root_span);
        span["span_kind"] = json!("receipt_deref");
        span["span_name"] = json!(format!("final-packet.prove.deref.{label}"));
        span["receipt_path"] = json!(path);
        span["dereferenced_receipt_label"] = json!(label);
        span["dereferenced_receipt_status"] = json!(status);
        span["dereferenced_receipt_digest"] = json!(digest);
        span["child_spans"] = Value::Array(Vec::new());
        children.push(span);
    }
    obs["trace_bundle_digest"] = json!(crate::digest::canonical_json(&obs["trace"]));
}

fn dereferenced_receipts(receipt: &Value) -> Vec<(String, String, String, String)> {
    let mut out = Vec::new();
    for key in [
        "cli_performance",
        "registry_exposure",
        "source_audit",
        "coverage",
    ] {
        if let Some(row) = receipt.get(key) {
            out.push(span_row(key, row));
        }
    }
    for (index, row) in receipt
        .get("package_receipts")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .enumerate()
    {
        out.push(span_row(&format!("package_receipt_{index}"), row));
    }
    out
}

fn span_row(label: &str, row: &Value) -> (String, String, String, String) {
    (
        label.to_string(),
        text(row, "path").to_string(),
        text(row, "status").to_string(),
        text(row, "digest").to_string(),
    )
}

fn child_span_id(root_span: &str, label: &str) -> String {
    let digest = crate::digest::bytes(format!("{root_span}:final-packet:{label}").as_bytes());
    format!("span-{}", &digest["sha256:".len()..34])
}

fn text<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or("")
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
