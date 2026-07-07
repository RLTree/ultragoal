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
    let subcommand = format!(
        "-lc {}",
        crate::cli::live_loop::nodes::measurement::full_command::runtime_command_text(
            surface.narrow_rerun
        )
    );
    let observation = crate::cli::observe::telemetry::command_receipt_for_candidate(
        root,
        crate::cli::observe::telemetry::CommandTelemetry {
            command: "bash",
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::live_loop::surfaces::surface_by_id;

    fn command() -> LiveLoopCommand {
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

    fn command_run(exit_code: i32, status_success: bool) -> FullCommandRun {
        FullCommandRun {
            exit_code,
            status_success,
            launch_error: false,
            duration_ms: 23,
            stdout_digest: "sha256:stdout".to_string(),
            stderr_digest: "sha256:stderr".to_string(),
            failure: Default::default(),
        }
    }

    #[test]
    fn command_observation_records_failed_verified_work_without_speed_claim() {
        let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
            "live-loop-command-observation-fail",
        );
        std::fs::create_dir_all(&root).expect("root");
        crate::json_boundary::write_json(
            &root.join("plugin-manifest-draft.json"),
            &serde_json::json!({"resources":["plugin-manifest-draft.json"]}),
        )
        .expect("manifest");
        let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
        let surface = surface_by_id("changed_files").expect("surface");
        let receipt = receipt_path(surface.id);
        let value = write(
            &root,
            surface,
            &candidate,
            &command(),
            &command_run(2, false),
            &receipt,
        )
        .expect("failed command observation");
        assert_eq!(value.value["status"], "fail");
        assert_eq!(
            value.value["event"]["failure_class"],
            "verified_local_command_failed"
        );
        assert_eq!(
            value.value["event"]["claim_impact"],
            "source_local_live_loop_node_observation_only_not_speed_claim"
        );
        std::fs::remove_dir_all(root).expect("cleanup failed command observation");
    }

    #[test]
    fn command_observation_rejects_external_receipt_path() {
        let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
            "live-loop-command-observation-path",
        );
        std::fs::create_dir_all(&root).expect("root");
        crate::json_boundary::write_json(
            &root.join("plugin-manifest-draft.json"),
            &serde_json::json!({"resources":["plugin-manifest-draft.json"]}),
        )
        .expect("manifest");
        let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
        let surface = surface_by_id("changed_files").expect("surface");
        let err = write(
            &root,
            surface,
            &candidate,
            &command(),
            &command_run(0, true),
            Path::new("/tmp/live-loop-command-observation.json"),
        )
        .expect_err("absolute observation receipt rejected");
        assert!(err.contains("external debug only") || err.contains("outside root"));
        std::fs::remove_dir_all(root).expect("cleanup rejected command observation");
    }

    #[test]
    fn command_observation_identity_parser_fails_closed() {
        let err = CommandObservation::from_receipt(serde_json::json!({
            "run_id": "run-present",
            "correlation_id": "corr-present"
        }))
        .expect_err("missing trace id rejected");

        assert_eq!(err, "live-loop command observation missing trace_id");
    }
}
