use crate::cli::observe::types::{ObserveCommand, ObserveOperation};
use serde_json::json;

pub(crate) fn query_text(command: &ObserveCommand) -> String {
    match command.operation {
        ObserveOperation::MetricsQuery => metric_query_text(command),
        ObserveOperation::TracesQuery => trace_tags(command),
        _ => log_query_text(command),
    }
}

fn log_query_text(command: &ObserveCommand) -> String {
    command.query.clone().unwrap_or_else(|| {
        command
            .run_id
            .as_ref()
            .map(|value| log_field("run_id", value))
            .or_else(|| {
                command
                    .law_id
                    .as_ref()
                    .map(|value| log_field("law_id", value))
            })
            .or_else(|| {
                command
                    .check_id
                    .as_ref()
                    .map(|value| log_field("check_id", value))
            })
            .or_else(|| {
                command
                    .claim_id
                    .as_ref()
                    .map(|value| log_field("claim_id", value))
            })
            .unwrap_or_else(|| "*".to_string())
    })
}

fn metric_query_text(command: &ObserveCommand) -> String {
    command.query.clone().unwrap_or_else(|| {
        metric_filter(command)
            .map(|filter| format!("ultragoal_command_total{{{filter}}}"))
            .unwrap_or_else(|| "ultragoal_command_total".to_string())
    })
}

fn metric_filter(command: &ObserveCommand) -> Option<String> {
    command
        .run_id
        .as_ref()
        .map(|value| metric_label("run_id", value))
        .or_else(|| {
            command
                .law_id
                .as_ref()
                .map(|value| metric_label("law_id", value))
        })
        .or_else(|| {
            command
                .check_id
                .as_ref()
                .map(|value| metric_label("check_id", value))
        })
        .or_else(|| {
            command
                .claim_id
                .as_ref()
                .map(|value| metric_label("claim_id", value))
        })
}

fn metric_label(label: &str, value: &str) -> String {
    format!("{label}=\"{}\"", prom_escape(value))
}

fn prom_escape(value: &str) -> String {
    value
        .chars()
        .flat_map(|ch| match ch {
            '\\' => "\\\\".chars().collect::<Vec<_>>(),
            '"' => "\\\"".chars().collect::<Vec<_>>(),
            '\n' => "\\n".chars().collect::<Vec<_>>(),
            other => vec![other],
        })
        .collect()
}

fn log_field(field: &str, value: &str) -> String {
    format!("{field}:{}", log_escape(value))
}

fn log_escape(value: &str) -> String {
    value.replace('"', "").replace('\n', "")
}

pub(crate) fn trace_tags(command: &ObserveCommand) -> String {
    if let Some(run_id) = command.run_id.as_deref() {
        json!({"run_id": run_id}).to_string()
    } else if let Some(law_id) = command.law_id.as_deref() {
        json!({"law_id": law_id}).to_string()
    } else if let Some(check_id) = command.check_id.as_deref() {
        json!({"check_id": check_id}).to_string()
    } else if let Some(claim_id) = command.claim_id.as_deref() {
        json!({"claim_id": claim_id}).to_string()
    } else {
        "{}".to_string()
    }
}
