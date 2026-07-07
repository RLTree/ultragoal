use super::super::nodes::status::MeasurementState;
use super::super::nodes::timing::NodeTiming;
use super::super::surfaces::{BOUNDARY_PROOF_POLICY, LoopValidationSurface};
use serde_json::{Map, Value, json};

pub(super) fn insert(
    object: &mut Map<String, Value>,
    surface: LoopValidationSurface,
    node_timing: Option<&NodeTiming>,
    measurement: &MeasurementState,
) {
    let (claim_name, proof_surface, reconciliation_surface, claim_status) = if surface
        .high_frequency
    {
        (
            "source-local live-loop speed claim",
            proof_surface_for(node_timing),
            "same-candidate logs, metrics, traces, explain output, and timing receipt reconciliation",
            if measurement.status == "pass" {
                "supported_source_local"
            } else {
                "blocked"
            },
        )
    } else if surface.hot_loop_policy == BOUNDARY_PROOF_POLICY {
        (
            "strict boundary proof claim",
            "withheld from dirty hot-loop execution; requires canonical boundary command on current candidate",
            "same-candidate receipt/stdout/telemetry reconciliation at the source-local proof boundary",
            "withheld_until_boundary",
        )
    } else {
        (
            "diagnostic live-loop context observation",
            "current candidate/context projection only; not a speed proof",
            "none required because the row is explicitly non-claimable",
            "observation_only",
        )
    };
    object.insert("claim_name".to_string(), json!(claim_name));
    object.insert(
        "product_behavior_observed".to_string(),
        json!(surface.command),
    );
    object.insert("proof_surface".to_string(), json!(proof_surface));
    object.insert(
        "independent_reconciliation_surface".to_string(),
        json!(reconciliation_surface),
    );
    object.insert("claim_status".to_string(), json!(claim_status));
}

fn proof_surface_for(timing: Option<&NodeTiming>) -> &'static str {
    match timing.map(|row| row.proof_kind.as_str()) {
        Some("executed") => {
            "executed current-candidate command with exit status, work units, digests, and timing receipt"
        }
        Some("verified_cache_hit") => {
            "verified same-candidate cache replay with current input digests and equivalence proof"
        }
        Some(_) => "invalid proof_kind; row is blocked",
        None => "missing timing proof; row is blocked",
    }
}
