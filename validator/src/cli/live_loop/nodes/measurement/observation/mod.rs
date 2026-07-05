mod command_event;
mod diagnostics;
mod query_roundtrip;

use super::full_command::FullCommandRun;
use crate::cli::live_loop::{LiveLoopCommand, surfaces::LoopValidationSurface};
use serde_json::{Value, json};
use std::path::Path;

#[derive(Clone, Debug)]
pub(crate) struct TelemetryReconciliation {
    pub(crate) status: String,
    pub(crate) value: Value,
}

impl TelemetryReconciliation {
    pub(crate) fn value(&self) -> Value {
        self.value.clone()
    }

    pub(crate) fn failure_summary(
        &self,
    ) -> crate::cli::live_loop::nodes::command_failure::CommandFailureSummary {
        crate::cli::live_loop::nodes::command_failure::CommandFailureSummary::from_value(
            self.value.get("command_observation"),
        )
    }
}

pub(crate) fn reconcile(
    root: &Path,
    surface: LoopValidationSurface,
    candidate: &str,
    command: &LiveLoopCommand,
    actual_work: &FullCommandRun,
) -> TelemetryReconciliation {
    match reconcile_result(root, surface, candidate, command, actual_work) {
        Ok(value) => TelemetryReconciliation {
            status: status_from_value(&value).to_string(),
            value,
        },
        Err(err) => TelemetryReconciliation {
            status: "command_observation_failed".to_string(),
            value: json!({
                "status": "command_observation_failed",
                "failure": err,
                "claim_impact": "live_loop_node_timing_blocked"
            }),
        },
    }
}

fn reconcile_result(
    root: &Path,
    surface: LoopValidationSurface,
    candidate: &str,
    command: &LiveLoopCommand,
    actual_work: &FullCommandRun,
) -> Result<Value, String> {
    let observation_path = command_event::receipt_path(surface.id);
    let observation = command_event::write(
        root,
        surface,
        candidate,
        command,
        actual_work,
        &observation_path,
    )?;
    let run_id = text(&observation, "run_id")?.to_string();
    let correlation_id = text(&observation, "correlation_id")?.to_string();
    let trace_id = text(&observation, "trace_id")?.to_string();
    let logs = query_roundtrip::run(
        root,
        surface,
        crate::cli::observe::types::ObserveOperation::LogsQuery,
        &run_id,
        &correlation_id,
    )?;
    let metrics = query_roundtrip::run(
        root,
        surface,
        crate::cli::observe::types::ObserveOperation::MetricsQuery,
        &run_id,
        &correlation_id,
    )?;
    let traces = query_roundtrip::run(
        root,
        surface,
        crate::cli::observe::types::ObserveOperation::TracesQuery,
        &run_id,
        &correlation_id,
    )?;
    let explain = query_roundtrip::run(
        root,
        surface,
        crate::cli::observe::types::ObserveOperation::ExplainFailure,
        &run_id,
        &correlation_id,
    )?;
    let status = if [&logs, &metrics, &traces, &explain]
        .iter()
        .all(|result| result.status == "pass")
    {
        "pass"
    } else {
        "query_or_explain_reconciliation_failed"
    };
    Ok(json!({
        "status": status,
        "run_id": run_id,
        "correlation_id": correlation_id,
        "trace_id": trace_id,
        "command_observation_receipt": observation_path.display().to_string(),
        "command_observation": observation,
        "logs_query": logs.value(),
        "metrics_query": metrics.value(),
        "traces_query": traces.value(),
        "explain_failure": explain.value(),
        "claim_impact": "source_local_live_loop_node_observation_only_not_speed_claim"
    }))
}

fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("live-loop command observation missing {key}"))
}

fn status_from_value(value: &Value) -> &str {
    value
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("fail")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::live_loop::surfaces::surface_by_id;
    use crate::self_tests::boundaries::workspace_fixtures::temp_root;
    use std::path::PathBuf;

    #[test]
    fn command_observation_receipt_names_actual_node_command_surface() {
        let root = temp_root("live-loop-command-observation");
        std::fs::create_dir_all(&root).expect("root");
        crate::json_boundary::write_json(
            &root.join("plugin-manifest-draft.json"),
            &json!({"resources":["plugin-manifest-draft.json"]}),
        )
        .expect("manifest");
        let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
        let surface = surface_by_id("changed_files").expect("surface");
        let command = LiveLoopCommand {
            action: crate::cli::live_loop::LiveLoopAction::Measure,
            tier: "hot".to_string(),
            cache_mode: "verified-local".to_string(),
            jobs: None,
            receipt: PathBuf::from("validation_artifacts/observability/live-loop-node-timing.json"),
            node_id: Some("changed_files".to_string()),
            measure_all: false,
        };
        let run = FullCommandRun {
            exit_code: 0,
            status_success: true,
            launch_error: false,
            duration_ms: 7,
            stdout_digest: "sha256:stdout".to_string(),
            stderr_digest: "sha256:stderr".to_string(),
            failure: Default::default(),
        };
        let receipt = command_event::receipt_path(surface.id);
        let value = command_event::write(&root, surface, &candidate, &command, &run, &receipt)
            .expect("command observation");

        assert_eq!(value["status"], "pass");
        assert_eq!(value["event"]["operation"], "loop.measure.changed_files");
        assert_eq!(value["event"]["surface"], "candidate_delta");
        assert_eq!(value["event"]["duration_ms"], 7);
        assert_eq!(
            value["event"]["claim_impact"],
            "source_local_live_loop_node_observation_only_not_speed_claim"
        );
        assert!(root.join(receipt).is_file());
        std::fs::remove_dir_all(root).expect("cleanup command observation");
    }
}
