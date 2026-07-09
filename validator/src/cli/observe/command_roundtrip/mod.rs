use crate::audit::observability::specs::{self, CommandObservabilitySpec};
use crate::cli::observe::query::{self, QueryKind};
use crate::cli::observe::types::{ObserveCommand, ObserveOperation};
use serde_json::Value;
use std::path::Path;
use std::time::Instant;

mod checks;
mod expected;
mod guards;
mod identity;
mod live_rows;
mod process;
mod receipt;
mod reconciliation;
mod row;
#[cfg(test)]
mod tests;

#[derive(Clone, Debug)]
pub(super) struct CommandRoundtripRecord {
    observable: bool,
    value: Value,
}

impl CommandRoundtripRecord {
    fn new(observable: bool, value: Value) -> Self {
        Self { observable, value }
    }

    pub(super) fn is_observable(&self) -> bool {
        self.observable
    }

    #[cfg(test)]
    pub(super) fn as_json(&self) -> &Value {
        &self.value
    }

    pub(super) fn into_json(self) -> Value {
        self.value
    }
}

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
    receipt::build(root, command, candidate, results, started)
}

fn requested_specs(command: &ObserveCommand) -> Result<Vec<CommandObservabilitySpec>, String> {
    match (&command.target_command, &command.target_family) {
        (Some(_), Some(_)) => {
            Err("observe command-roundtrip accepts either --command or --family, not both".into())
        }
        (Some(id), None) => specs::command(id)
            .map(|spec| vec![spec])
            .ok_or_else(|| format!("unknown observability command spec: {id}")),
        (None, Some(id)) => {
            let family = specs::family(id)
                .ok_or_else(|| format!("unknown observability family spec: {id}"))?;
            Ok(specs::family_commands(family))
        }
        (None, None) => {
            Err("observe command-roundtrip requires --command <id> or --family <family>".into())
        }
    }
}

fn run_command_roundtrip(
    root: &Path,
    spec: CommandObservabilitySpec,
    timeout_ms: u64,
) -> Result<CommandRoundtripRecord, String> {
    run_command_roundtrip_with(root, spec, timeout_ms, process::run_production_command)
}

fn run_command_roundtrip_with<F>(
    root: &Path,
    spec: CommandObservabilitySpec,
    timeout_ms: u64,
    run_production: F,
) -> Result<CommandRoundtripRecord, String>
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
    let reconciliation = reconciliation::report_for_spec(
        &production,
        &command_receipt,
        &logs,
        &metrics,
        &traces,
        &explain,
        spec,
    );
    let observable = checks::is_command_observable(
        &production,
        &command_receipt,
        &logs,
        &metrics,
        &traces,
        &explain,
    ) && reconciliation.is_reconciled();
    Ok(CommandRoundtripRecord::new(
        observable,
        row::build(
            spec,
            &production,
            &command_receipt,
            [&logs, &metrics, &traces, &explain],
            &reconciliation,
            observable,
        ),
    ))
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
    receipt::write_generated_roundtrip(root, &command.receipt_rel(), "query", &value)?;
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
    receipt::write_generated_roundtrip(root, &command.receipt_rel(), "explain", &value)?;
    Ok(value)
}
