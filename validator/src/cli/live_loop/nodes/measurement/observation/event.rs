use super::diagnostics;
use crate::cli::live_loop::{
    LiveLoopCommand, nodes::measurement::full_command::FullCommandRun,
    nodes::measurement::observation::query, surfaces::LoopValidationSurface,
};
use serde_json::{Value, json};
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
    let mut observation = crate::cli::observe::telemetry::command_receipt_for_candidate(
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
    insert_command_result_authority(&mut observation, actual_work);
    bind_process_result_authority(&mut observation, surface, actual_work, &argv);
    let observation = CommandObservation::from_receipt(observation)?;
    write_receipt(root, receipt_path, &observation.value)?;
    Ok(observation)
}

fn insert_command_result_authority(observation: &mut Value, actual_work: &FullCommandRun) {
    let output_digest = crate::digest::bytes(
        format!(
            "stdout={};stderr={}",
            actual_work.stdout_digest, actual_work.stderr_digest
        )
        .as_bytes(),
    );
    let result_digest = crate::digest::bytes(
        format!(
            "exit={};launch={};output={output_digest}",
            actual_work.exit_code, actual_work.launch_error
        )
        .as_bytes(),
    );
    let authority = json!({
        "authority": "live_loop_command_observation_actual_work",
        "exit_status": actual_work.exit_code,
        "launch_error": actual_work.launch_error,
        "stdout_digest": actual_work.stdout_digest,
        "stderr_digest": actual_work.stderr_digest,
        "output_digest": output_digest,
        "result_digest": result_digest,
        "executed_test_count": actual_work.executed_test_count
    });
    if let Some(object) = observation.as_object_mut() {
        object.insert("command_result_authority".to_string(), authority.clone());
        if let Some(event) = object.get_mut("event").and_then(Value::as_object_mut) {
            event.insert("command_result_authority".to_string(), authority);
        }
    }
}

fn bind_process_result_authority(
    observation: &mut Value,
    surface: LoopValidationSurface,
    actual_work: &FullCommandRun,
    argv: &[String],
) {
    let output_digest = crate::digest::bytes(
        format!(
            "stdout={};stderr={}",
            actual_work.stdout_digest, actual_work.stderr_digest
        )
        .as_bytes(),
    );
    let result_digest = crate::digest::bytes(
        format!(
            "exit={};launch={};output={output_digest}",
            actual_work.exit_code, actual_work.launch_error
        )
        .as_bytes(),
    );
    let object = observation
        .as_object_mut()
        .expect("command telemetry receipt is always an object");
    object.insert(
        "command_identity".to_string(),
        serde_json::json!({
            "node_id": surface.id,
            "canonical_full_command": surface.canonical_full_command,
            "verified_local_command": super::super::full_command::product_command_text(surface.narrow_rerun),
            "command_argv": argv,
        }),
    );
    object.insert(
        "process_result_authority".to_string(),
        serde_json::json!({
            "exit_status": actual_work.exit_code,
            "status_success": actual_work.status_success,
            "launch_error": actual_work.launch_error,
            "duration_ms": actual_work.duration_ms,
            "work_unit_count": 1,
            "stdout_digest": actual_work.stdout_digest,
            "stderr_digest": actual_work.stderr_digest,
            "output_digest": output_digest,
            "result_digest": result_digest,
            "redaction_status": "pass",
            "bounded_output_status": "digest_only_raw_output_not_retained",
            "claim_ceiling": "source_local_command_result_authority_only",
            "unsupported_claims": [
                "readiness",
                "release",
                "completion",
                "final_packet_correctness",
                "update_goal_eligibility"
            ]
        }),
    );
}

fn telemetry_command_name(command_text: &str, argv: &[String]) -> String {
    let program = argv
        .first()
        .expect("product_command_argv always returns a command program");
    if command_text.starts_with("target/debug/ultragoal") || program.ends_with("/ultragoal") {
        return "ultragoal".to_string();
    }
    std::path::Path::new(program)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(program)
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
