use crate::audit::observability::specs::{self, CommandObservabilitySpec};
use crate::cli::observe::query::{self, QueryKind};
use crate::cli::observe::types::{ObserveCommand, ObserveOperation};
use serde_json::{Value, json};
use std::path::Path;
use std::time::Instant;

mod checks;
mod process;
#[cfg(test)]
mod tests;

pub(crate) fn run(root: &Path, command: &ObserveCommand) -> Result<Value, String> {
    run_with_timeout(root, command, command.timeout_ms.max(30_000))
}

fn run_with_timeout(
    root: &Path,
    command: &ObserveCommand,
    roundtrip_timeout_ms: u64,
) -> Result<Value, String> {
    let started = Instant::now();
    let candidate = crate::package::inventory::package_digest(root)?;
    let commands = requested_specs(command)?;
    let mut results = Vec::new();
    for spec in commands {
        results.push(run_command_roundtrip(root, spec, roundtrip_timeout_ms)?);
    }
    receipt(root, command, candidate, results, started)
}

fn requested_specs(command: &ObserveCommand) -> Result<Vec<CommandObservabilitySpec>, String> {
    match (&command.target_command, &command.target_family) {
        (Some(_), Some(_)) => {
            Err("observe fit accepts either --command or --family, not both".into())
        }
        (Some(id), None) => specs::command(id)
            .map(|spec| vec![spec])
            .ok_or_else(|| format!("unknown observability command spec: {id}")),
        (None, Some(id)) => {
            let family = specs::family(id)
                .ok_or_else(|| format!("unknown observability family spec: {id}"))?;
            Ok(specs::family_commands(family))
        }
        (None, None) => Err("observe fit requires --command <id> or --family <family>".into()),
    }
}

fn run_command_roundtrip(
    root: &Path,
    spec: CommandObservabilitySpec,
    timeout_ms: u64,
) -> Result<Value, String> {
    run_command_roundtrip_with(root, spec, timeout_ms, process::run_production_command)
}

fn run_command_roundtrip_with<F>(
    root: &Path,
    spec: CommandObservabilitySpec,
    timeout_ms: u64,
    run_production: F,
) -> Result<Value, String>
where
    F: Fn(&Path, &[&str]) -> Result<process::CommandOutput, String>,
{
    let production = run_production(root, spec.command_args)?;
    let command_receipt = crate::json_boundary::read_json(&root.join(spec.receipt_rel))?;
    let run_id = checks::text(&command_receipt, "run_id")?;
    let correlation_id = checks::text(&command_receipt, "correlation_id")?;
    let logs_result = query_roundtrip(
        root,
        spec,
        QueryKind::Logs,
        run_id,
        correlation_id,
        timeout_ms,
    );
    let logs = logs_result?;
    let metrics_result = query_roundtrip(
        root,
        spec,
        QueryKind::Metrics,
        run_id,
        correlation_id,
        timeout_ms,
    );
    let metrics = metrics_result?;
    let traces_result = query_roundtrip(
        root,
        spec,
        QueryKind::Traces,
        run_id,
        correlation_id,
        timeout_ms,
    );
    let traces = traces_result?;
    let explain = explain_roundtrip(root, spec, run_id, correlation_id, timeout_ms)?;
    let observable = checks::is_command_observable(
        &production,
        &command_receipt,
        &logs,
        &metrics,
        &traces,
        &explain,
    );
    Ok(json!({
        "command_id": spec.id,
        "family": spec.family,
        "operation": spec.operation,
        "roundtrip_status": roundtrip_status(observable),
        "production_exit_status": production.exit_code,
        "production_stdout": production.stdout,
        "production_stderr": production.stderr,
        "receipt_path": spec.receipt_rel,
        "validator_check_id": spec.validator_check_id,
        "query_roundtrip_paths": checks::query_paths(spec),
        "explain_roundtrip_path": checks::roundtrip_path(spec, "explain-failure"),
        "stdout_receipt_same_candidate": checks::same_candidate(&command_receipt, &logs, &metrics, &traces, &explain),
        "logs_query_status": checks::status(&logs),
        "metrics_query_status": checks::status(&metrics),
        "traces_query_status": checks::status(&traces),
        "explain_status": checks::status(&explain),
        "claim_impact": spec.claim_impact
    }))
}

fn roundtrip_status(observable: bool) -> &'static str {
    if observable { "observable" } else { "partial" }
}

fn query_roundtrip(
    root: &Path,
    spec: CommandObservabilitySpec,
    kind: QueryKind,
    run_id: &str,
    correlation_id: &str,
    timeout_ms: u64,
) -> Result<Value, String> {
    let operation = match kind {
        QueryKind::Logs => ObserveOperation::LogsQuery,
        QueryKind::Metrics => ObserveOperation::MetricsQuery,
        QueryKind::Traces => ObserveOperation::TracesQuery,
    };
    let command = ObserveCommand {
        operation,
        receipt: Some(checks::roundtrip_path(spec, kind.label())),
        query: None,
        run_id: Some(run_id.to_string()),
        correlation_id: Some(correlation_id.to_string()),
        claim_id: None,
        check_id: None,
        law_id: None,
        target_command: None,
        target_family: None,
        row_limit: 100,
        byte_limit: 262_144,
        timeout_ms,
    };
    let value = query::run(root, &command, kind)?;
    crate::json_boundary::write_json(&root.join(command.receipt_rel()), &value)?;
    Ok(value)
}

fn explain_roundtrip(
    root: &Path,
    spec: CommandObservabilitySpec,
    run_id: &str,
    correlation_id: &str,
    timeout_ms: u64,
) -> Result<Value, String> {
    let command = ObserveCommand {
        operation: ObserveOperation::ExplainFailure,
        receipt: Some(checks::roundtrip_path(spec, "explain-failure")),
        query: None,
        run_id: Some(run_id.to_string()),
        correlation_id: Some(correlation_id.to_string()),
        claim_id: None,
        check_id: None,
        law_id: None,
        target_command: None,
        target_family: None,
        row_limit: 100,
        byte_limit: 262_144,
        timeout_ms,
    };
    let value = super::explain::run(root, &command)?;
    crate::json_boundary::write_json(&root.join(command.receipt_rel()), &value)?;
    Ok(value)
}

fn receipt(
    root: &Path,
    command: &ObserveCommand,
    candidate: String,
    results: Vec<Value>,
    started: Instant,
) -> Result<Value, String> {
    let observable = results
        .iter()
        .all(|row| row["roundtrip_status"] == "observable");
    let status = if observable { "pass" } else { "fail" };
    let failure = (!observable).then_some("observability command roundtrip is incomplete");
    let mut receipt = super::telemetry::base_receipt(root, command, status, failure)?;
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
    receipt["results"] = json!(results);
    receipt["claim_ceiling"] = json!(
        "source-local observability command roundtrip only; Gate 92 remains blocked until every row has same-candidate telemetry reconciliation"
    );
    receipt["claim_impact"] = json!(
        "supports one spec-driven command observability roundtrip increment only_not_readiness_release_completion_update_goal"
    );
    receipt["supported_claims"] = json!(["spec_driven_observability_command_roundtrip_increment"]);
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
