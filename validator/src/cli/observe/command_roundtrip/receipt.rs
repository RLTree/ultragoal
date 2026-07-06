use crate::audit::observability::specs;
use crate::cli::observe::types::ObserveCommand;
use serde_json::{Value, json};
use std::path::Path;
use std::time::Instant;

use super::CommandRoundtripRecord;

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
