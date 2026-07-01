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
        let mut fields = Vec::new();
        if let Some(value) = command.run_id.as_ref() {
            fields.push(log_field("run_id", value));
        }
        if let Some(value) = command.correlation_id.as_ref() {
            fields.push(log_field("correlation_id", value));
        }
        if !fields.is_empty() {
            return fields.join(" ");
        }
        command
            .law_id
            .as_ref()
            .map(|value| log_field("law_id", value))
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
    command
        .query
        .clone()
        .unwrap_or_else(|| bounded_metric_query(&metric_selector(metric_filter(command))))
}

fn bounded_metric_query(selector: &str) -> String {
    format!(
        "sum by (__name__,operation,status,check_id,claim_id,surface,failure_class,exporter,saturation_status) (max_over_time({selector}[5m]))"
    )
}

pub(crate) fn bounded_metric_query_for_operation(operation: &str) -> String {
    bounded_metric_query(&metric_selector(metric_filter_with_primary(vec![
        metric_label("operation", operation),
    ])))
}

pub(crate) fn bounded_failure_metric_query_for_operation(operation: &str) -> String {
    bounded_metric_query(&metric_selector(metric_filter_with_primary(vec![
        metric_label("operation", operation),
        metric_label("status", "fail"),
    ])))
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
    metric_filter_with_primary(primary.into_iter().collect())
}

fn metric_filter_with_primary(mut fields: Vec<String>) -> String {
    fields.retain(|field| !field.is_empty());
    fields.join(",")
}

fn metric_selector(filter: String) -> String {
    let names = metric_label(
        "__name__",
        "~ultragoal_command_total|ultragoal_command_duration_ms|ultragoal_command_task_count|ultragoal_command_queue_depth",
    );
    let names = names.replacen("__name__=\"~", "__name__=~\"", 1);
    let filter = metric_filter_with_primary(vec![names, filter]);
    format!("{{{filter}}}")
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

fn trace_tag_pairs(command: &ObserveCommand) -> Vec<(&'static str, &str)> {
    let mut pairs = Vec::new();
    if let Some(run_id) = command.run_id.as_deref() {
        pairs.push(("run_id", run_id));
    }
    if let Some(correlation_id) = command.correlation_id.as_deref() {
        pairs.push(("correlation_id", correlation_id));
    }
    if pairs.is_empty() {
        if let Some(law_id) = command.law_id.as_deref() {
            pairs.push(("law_id", law_id));
        } else if let Some(check_id) = command.check_id.as_deref() {
            pairs.push(("check_id", check_id));
        } else if let Some(claim_id) = command.claim_id.as_deref() {
            pairs.push(("claim_id", claim_id));
        }
    }
    pairs
}

pub(crate) fn trace_tags(command: &ObserveCommand) -> String {
    let pairs = trace_tag_pairs(command);
    if pairs.is_empty() {
        "{}".to_string()
    } else {
        let mut tags = serde_json::Map::new();
        for (key, value) in pairs {
            tags.insert(key.to_string(), json!(value));
        }
        serde_json::Value::Object(tags).to_string()
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
            correlation_id: None,
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
        assert_eq!(
            query,
            "sum by (__name__,operation,status,check_id,claim_id,surface,failure_class,exporter,saturation_status) (max_over_time({__name__=~\"ultragoal_command_total|ultragoal_command_duration_ms|ultragoal_command_task_count|ultragoal_command_queue_depth\",check_id=\"coverage-prove-observability-binding\"}[5m]))"
        );
    }

    #[test]
    fn metrics_query_does_not_use_run_id_as_label() {
        let mut command = command(ObserveOperation::MetricsQuery);
        command.run_id = Some("run-abc".to_string());

        let query = query_text(&command);
        assert!(!query.contains("run_id=\"run-abc\""));
        assert!(!query.contains("run_id="));
    }

    #[test]
    fn logs_and_traces_can_select_run_and_correlation_without_metric_labels() {
        let mut logs = command(ObserveOperation::LogsQuery);
        logs.run_id = Some("run-abc".to_string());
        logs.correlation_id = Some("corr-abc".to_string());
        assert_eq!(query_text(&logs), "run_id:run-abc correlation_id:corr-abc");

        let mut traces = command(ObserveOperation::TracesQuery);
        traces.run_id = Some("run-abc".to_string());
        traces.correlation_id = Some("corr-abc".to_string());
        assert_eq!(
            query_text(&traces),
            json!({"correlation_id": "corr-abc", "run_id": "run-abc"}).to_string()
        );

        let mut metrics = command(ObserveOperation::MetricsQuery);
        metrics.run_id = Some("run-abc".to_string());
        metrics.correlation_id = Some("corr-abc".to_string());
        let query = query_text(&metrics);
        assert_eq!(
            query,
            "sum by (__name__,operation,status,check_id,claim_id,surface,failure_class,exporter,saturation_status) (max_over_time({__name__=~\"ultragoal_command_total|ultragoal_command_duration_ms|ultragoal_command_task_count|ultragoal_command_queue_depth\"}[5m]))"
        );
        assert!(!query.contains("run_id=\"run-abc\""));
        assert!(!query.contains("correlation_id=\"corr-abc\""));
    }

    #[test]
    fn custom_metrics_query_is_preserved() {
        let mut command = command(ObserveOperation::MetricsQuery);
        command.query = Some("ultragoal_command_total".to_string());

        assert_eq!(query_text(&command), "ultragoal_command_total");
    }
}
