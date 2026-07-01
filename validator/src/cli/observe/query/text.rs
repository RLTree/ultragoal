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
        range_query(&format!(
            "ultragoal_command_total{{{}}}",
            metric_filter(command)
        ))
    })
}

fn range_query(selector: &str) -> String {
    format!("max_over_time({selector}[24h])")
}

pub(crate) fn bounded_metric_query_for_operation(operation: &str) -> String {
    range_query(&format!(
        "ultragoal_command_total{{{}}}",
        metric_filter_with_primary(Some(metric_label("operation", operation)))
    ))
}

fn metric_filter(command: &ObserveCommand) -> String {
    let primary = command
        .law_id
        .as_ref()
        .map(|value| metric_label("law_id", value))
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
        });
    metric_filter_with_primary(primary)
}

fn metric_filter_with_primary(primary: Option<String>) -> String {
    let mut filters = Vec::new();
    if let Some(primary) = primary {
        filters.push(primary);
    }
    filters.extend(bounded_metric_absence_labels());
    filters.join(",")
}

fn bounded_metric_absence_labels() -> impl Iterator<Item = String> {
    [
        "candidate_digest",
        "run_id",
        "correlation_id",
        "trace_id",
        "span_id",
        "why_failed",
        "where_failed",
        "next_repair",
        "claim_impact",
    ]
    .into_iter()
    .map(|label| metric_label(label, ""))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::observe::types::{ObserveCommand, ObserveOperation};

    fn command(operation: ObserveOperation) -> ObserveCommand {
        ObserveCommand {
            operation,
            receipt: None,
            query: None,
            run_id: None,
            claim_id: None,
            check_id: None,
            law_id: None,
            row_limit: 100,
            byte_limit: 1024,
            timeout_ms: 1000,
        }
    }

    #[test]
    fn metrics_query_uses_bounded_historical_window() {
        let mut command = command(ObserveOperation::MetricsQuery);
        command.check_id = Some("coverage-prove-observability-binding".to_string());

        let query = query_text(&command);
        assert!(query.starts_with(
            "max_over_time(ultragoal_command_total{check_id=\"coverage-prove-observability-binding\","
        ));
        assert!(query.contains("candidate_digest=\"\""));
        assert!(query.contains("run_id=\"\""));
        assert!(query.ends_with("}[24h])"));
    }

    #[test]
    fn metrics_query_does_not_use_run_id_as_label() {
        let mut command = command(ObserveOperation::MetricsQuery);
        command.run_id = Some("run-abc".to_string());

        let query = query_text(&command);
        assert!(query.contains("run_id=\"\""));
        assert!(!query.contains("run_id=\"run-abc\""));
    }

    #[test]
    fn custom_metrics_query_is_preserved() {
        let mut command = command(ObserveOperation::MetricsQuery);
        command.query = Some("ultragoal_command_total".to_string());

        assert_eq!(query_text(&command), "ultragoal_command_total");
    }
}
