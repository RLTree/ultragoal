use crate::cli::observe::types::ObserveCommand;
use serde_json::Value;

pub(in crate::cli::observe::stdout) fn failure_metric_operation(
    command: &ObserveCommand,
    value: &Value,
) -> String {
    selected_text(value, "observed_operation")
        .or_else(|| selected_pointer(value, "/observed_run/operation"))
        .unwrap_or_else(|| command.operation.id())
        .to_string()
}

pub(in crate::cli::observe::stdout) fn failure_metric_query(
    command: &ObserveCommand,
    value: &Value,
) -> String {
    let operation = failure_metric_operation(command, value);
    match failure_metric_status(value) {
        Some(status) => crate::cli::observe::query::bounded_metric_query_for_operation_status(
            &operation, status,
        ),
        None => crate::cli::observe::query::bounded_failure_metric_query_for_operation(&operation),
    }
}

fn failure_metric_status(value: &Value) -> Option<&str> {
    selected_text(value, "observed_status")
        .or_else(|| selected_pointer(value, "/observed_run/status"))
        .or_else(|| selected_pointer(value, "/explanation_target/status"))
}

fn selected_text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    selected(value.get(key).and_then(Value::as_str))
}

fn selected_pointer<'a>(value: &'a Value, pointer: &str) -> Option<&'a str> {
    selected(value.pointer(pointer).and_then(Value::as_str))
}

fn selected(value: Option<&str>) -> Option<&str> {
    value.filter(|text| !text.trim().is_empty() && *text != "none")
}
