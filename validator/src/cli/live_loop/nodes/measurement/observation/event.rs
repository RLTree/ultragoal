use super::diagnostics;
use crate::cli::live_loop::{
    LiveLoopCommand, nodes::measurement::full_command::FullCommandRun,
    nodes::measurement::observation::query, surfaces::LoopValidationSurface,
};
use serde_json::Value;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub(super) struct CommandObservation {
    pub(super) value: Value,
    pub(super) run_id: String,
    pub(super) correlation_id: String,
    pub(super) trace_id: String,
}

impl CommandObservation {
    pub(super) fn from_receipt(value: Value) -> Result<Self, String> {
        Ok(Self {
            run_id: required_text(&value, "run_id")?.to_string(),
            correlation_id: required_text(&value, "correlation_id")?.to_string(),
            trace_id: required_text(&value, "trace_id")?.to_string(),
            value,
        })
    }

    pub(super) fn value(&self) -> Value {
        self.value.clone()
    }
}

pub(super) fn write(
    root: &Path,
    surface: LoopValidationSurface,
    candidate: &str,
    command: &LiveLoopCommand,
    actual_work: &FullCommandRun,
    receipt_path: &Path,
) -> Result<CommandObservation, String> {
    let failure_class = diagnostics::command_failure_class(actual_work);
    let status = if failure_class == "none" {
        "pass"
    } else {
        "fail"
    };
    let node_id = surface.id.replace('_', "-");
    let check_id = format!("live-loop-{node_id}-command-observation");
    let receipt_text = receipt_path.to_string_lossy();
    let argv = crate::cli::live_loop::nodes::measurement::full_command::product_command_argv(
        surface.narrow_rerun,
    );
    let command_name = telemetry_command_name(surface.narrow_rerun, &argv);
    let subcommand = argv.iter().skip(1).cloned().collect::<Vec<_>>().join(" ");
    let observation = crate::cli::observe::telemetry::command_receipt_for_candidate(
        root,
        crate::cli::observe::telemetry::CommandTelemetry {
            command: &command_name,
            subcommand: &subcommand,
            operation: &format!("loop.measure.{}", surface.id),
            surface: surface.surface,
            law_id: crate::cli::observe::types::LAW_ID,
            check_id: &check_id,
            claim_id: "live-loop-hot-repair-feedback",
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
    let observation = CommandObservation::from_receipt(observation)?;
    write_receipt(root, receipt_path, &observation.value)?;
    Ok(observation)
}

fn telemetry_command_name(command_text: &str, argv: &[String]) -> String {
    let Some(program) = argv.first() else {
        return "unknown-runtime-command".to_string();
    };
    if command_text.starts_with("target/debug/ultragoal") || program.ends_with("/ultragoal") {
        return "ultragoal".to_string();
    }
    std::path::Path::new(program)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown-runtime-command")
        .to_string()
}

pub(super) fn receipt_path(node_id: &str) -> PathBuf {
    query::receipt_path(node_id, "command-observation")
}

fn write_receipt(root: &Path, receipt_path: &Path, value: &Value) -> Result<(), String> {
    let path = crate::output_path::claim_artifact_path(
        root,
        receipt_path,
        "live loop command observation",
    )?;
    crate::json_boundary::write_json(&path, value)
}

fn required_text<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("live-loop command observation missing {key}"))
}
