use crate::audit::observability::specs;
use crate::cli::observe::types::ObserveCommand;
use serde_json::{Value, json};
use std::path::Path;
use std::time::Instant;

use super::CommandRoundtripRecord;

const QUERY_ROUNDTRIP_RECEIPT_LABEL: &str = "observe query roundtrip receipt";
const EXPLAIN_ROUNDTRIP_RECEIPT_LABEL: &str = "observe explain roundtrip receipt";

pub(super) fn build(
    root: &Path,
    command: &ObserveCommand,
    candidate: String,
    results: Vec<CommandRoundtripRecord>,
    started: Instant,
) -> Result<Value, String> {
    let observable = results.iter().all(CommandRoundtripRecord::is_observable);
    let rows: Vec<_> = results
        .into_iter()
        .map(CommandRoundtripRecord::into_json)
        .collect();
    let status = if observable { "pass" } else { "fail" };
    let failure = (!observable).then_some("observability command roundtrip is incomplete");
    let first_failure = (!observable).then(|| first_failure_row(&rows)).flatten();
    let mut receipt = super::super::telemetry::base_receipt(root, command, status, failure)?;
    receipt["schema"] = json!("harness-ultragoal.observe-roundtrip-receipt.v1");
    receipt["candidate_digest"] = json!(candidate);
    receipt["target_command"] = json!(command.target_command);
    receipt["target_family"] = json!(command.target_family);
    receipt["roundtrip_status"] = json!(if observable { "observable" } else { "partial" });
    receipt["duration_ms"] = json!(
        u64::try_from(started.elapsed().as_millis())
            .unwrap_or(u64::MAX)
            .max(1)
    );
    receipt["generated_from"] = json!("CommandObservabilitySpec and SurfaceObservabilitySpec");
    receipt["spec_command_ids"] = json!(specs::command_ids());
    receipt["results"] = json!(rows);
    receipt["claim_name"] = json!("source-local command telemetry roundtrip claim");
    receipt["product_behavior_observed"] =
        json!("real current-candidate ultragoal command runs captured by CommandObservabilitySpec");
    receipt["proof_surface"] = json!(
        "production stdout, command receipts, logs query receipts, metrics query receipts, traces query receipts, and explain receipts"
    );
    receipt["independent_reconciliation_surface"] = json!(
        "same-candidate run/correlation/digest reconciliation across stdout, receipts, logs, metrics, traces, and explain output"
    );
    receipt["claim_status"] = json!(if observable {
        "supported_source_local"
    } else {
        "partial_no_claim"
    });
    if let Some(row) = first_failure {
        receipt["failure_class"] = row_field(row, "failure_class", "command_roundtrip_partial");
        receipt["why_failed"] = row_field(row, "why_failed", "command roundtrip is incomplete");
        receipt["where_failed"] = row_field(row, "where_failed", "observe.command-roundtrip");
        receipt["next_repair"] = row_field(
            row,
            "next_repair",
            "repair the first partial command roundtrip row and rerun command-roundtrip",
        );
    }
    receipt["claim_ceiling"] = json!(
        "source-local observability command roundtrip only; observability product closure remains blocked until every row has same-candidate telemetry reconciliation"
    );
    receipt["claim_impact"] = json!(
        "supports one spec-driven command observability roundtrip increment only_not_readiness_release_completion_update_goal"
    );
    receipt["supported_claims"] = json!(if observable {
        vec!["spec_driven_observability_command_roundtrip_increment"]
    } else {
        Vec::<&str>::new()
    });
    receipt["blocked_claims"] = json!([
        "observability_product_closure",
        "readiness",
        "release",
        "completion",
        "final_packet_correctness",
        "update_goal_eligibility"
    ]);
    Ok(receipt)
}

fn first_failure_row(rows: &[Value]) -> Option<&Value> {
    rows.iter().find(|row| {
        row.get("roundtrip_status").and_then(Value::as_str) != Some("observable")
            || row
                .get("failure_class")
                .and_then(Value::as_str)
                .is_some_and(|class| class != "none")
    })
}

fn row_field(row: &Value, field: &str, fallback: &str) -> Value {
    row.get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty() && *value != "none")
        .map(|value| json!(value))
        .unwrap_or_else(|| json!(fallback))
}

pub(super) fn write_generated_roundtrip(
    root: &Path,
    rel: &Path,
    kind: &str,
    value: &Value,
) -> Result<(), String> {
    let label = match kind {
        "query" => QUERY_ROUNDTRIP_RECEIPT_LABEL,
        "explain" => EXPLAIN_ROUNDTRIP_RECEIPT_LABEL,
        _ => "observe command roundtrip receipt",
    };
    let path = crate::output_path::claim_artifact_path(root, rel, label)
        .expect("observe roundtrip receipt paths are generated package-relative paths");
    crate::json_boundary::write_json(&path, value)
}
