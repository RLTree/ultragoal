use super::diagnostics;
use crate::cli::live_loop::{
    LiveLoopCommand, nodes::measurement::full_command::FullCommandRun,
    nodes::measurement::observation::query_roundtrip, surfaces::LoopValidationSurface,
};
use serde_json::Value;
use std::path::{Path, PathBuf};

pub(super) fn write(
    root: &Path,
    surface: LoopValidationSurface,
    candidate: &str,
    command: &LiveLoopCommand,
    actual_work: &FullCommandRun,
    receipt_path: &Path,
) -> Result<Value, String> {
    let failure_class = diagnostics::command_failure_class(actual_work);
    let status = if failure_class == "none" {
        "pass"
    } else {
        "fail"
    };
    let node_id = surface.id.replace('_', "-");
    let check_id = format!("live-loop-{node_id}-command-observation");
    let receipt_text = receipt_path.to_string_lossy();
    let subcommand = format!("-lc {}", surface.narrow_rerun);
    let observation = crate::cli::observe::telemetry::command_receipt_for_candidate(
        root,
        crate::cli::observe::telemetry::CommandTelemetry {
            command: "bash",
            subcommand: &subcommand,
            operation: &format!("loop.measure.{}", surface.id),
            surface: surface.surface,
            law_id: crate::cli::observe::types::LAW_ID,
            check_id: &check_id,
            claim_id: "observability-live-loop-source-local-acceleration",
            artifact_path: command.receipt.to_string_lossy().as_ref(),
            receipt_path: receipt_text.as_ref(),
            status,
            failure_class,
            why_failed: &diagnostics::why_failed(surface, actual_work, failure_class),
            where_failed: &diagnostics::where_failed(surface, failure_class),
            next_repair: &diagnostics::next_repair(surface, failure_class),
            claim_impact: "source_local_live_loop_node_observation_only_not_speed_claim",
            blocked_claims: diagnostics::blocked_claims(),
            supported_claims: vec!["live_loop_node_command_observation".to_string()],
            runtime: Some(diagnostics::runtime(surface, command, actual_work)),
            emit: true,
        },
        candidate.to_string(),
    )?;
    write_receipt(root, receipt_path, &observation)?;
    Ok(observation)
}

pub(super) fn receipt_path(node_id: &str) -> PathBuf {
    query_roundtrip::receipt_path(node_id, "command-observation")
}

fn write_receipt(root: &Path, receipt_path: &Path, value: &Value) -> Result<(), String> {
    let path = crate::output_path::claim_artifact_path(
        root,
        receipt_path,
        "live loop command observation",
    )?;
    crate::json_boundary::write_json(&path, value)
}
