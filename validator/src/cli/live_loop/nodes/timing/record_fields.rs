use serde_json::Value;

use crate::cli::live_loop::surfaces::LoopValidationSurface;

pub(super) fn node_rows(value: &Value) -> Vec<&Value> {
    value
        .get("nodes")
        .and_then(Value::as_array)
        .map(|rows| rows.iter().collect())
        .unwrap_or_default()
}

pub(super) fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

pub(super) fn nonempty_text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    let text = text(value, key)?;
    (!text.is_empty()).then_some(text)
}

pub(super) fn positive(value: &Value, key: &str) -> Option<u64> {
    let number = value.get(key).and_then(Value::as_u64)?;
    (number > 0).then_some(number)
}

pub(super) fn i32_field(value: &Value, key: &str) -> Option<i32> {
    value
        .get(key)
        .and_then(Value::as_i64)
        .and_then(|number| i32::try_from(number).ok())
}

pub(super) fn valid_digest(value: &str) -> Option<&str> {
    (value.len() == 71
        && value.starts_with("sha256:")
        && value[7..].bytes().all(|byte| byte.is_ascii_hexdigit()))
    .then_some(value)
}

pub(super) fn has_nonempty_string_array(value: &Value, key: &str) -> bool {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|items| !items.is_empty() && items.iter().all(|item| item.as_str().is_some()))
        .unwrap_or(false)
}

pub(super) fn runtime_versions_match(value: &Value) -> bool {
    text(value, "validator_version") == Some(&crate::cli::live_loop::graph::validator_version())
        && text(value, "law_version") == Some(crate::cli::live_loop::graph::law_version())
        && text(value, "schema_version") == Some(crate::cli::live_loop::graph::schema_version())
        && text(value, "fixture_version") == Some(crate::cli::live_loop::graph::fixture_version())
        && text(value, "runtime_execution_model")
            == Some(crate::cli::live_loop::graph::runtime_execution_model())
}

pub(super) fn scheduler_contract_matches(value: &Value, surface: LoopValidationSurface) -> bool {
    if text(value, "graph_task_class") != Some(surface.execution_task_class.id()) {
        return false;
    }
    if text(value, "execution_task_class") != Some(surface.execution_task_class.id()) {
        return false;
    }
    if text(value, "execution_serial_reason") != Some(surface.execution_serial_reason) {
        return false;
    }
    if text(value, "worker_state") != Some("single_surface_measurement_worker")
        || text(value, "task_state") != Some("surface_measurement_completed")
        || text(value, "queue_state") != Some("deterministic_measurement_batch_order")
        || text(value, "executor_behavior")
            != Some("measure_surface_invokes_one_node_command_at_a_time")
        || text(value, "executor_scope")
            != Some("source_local_custom_tooling_prerequisite_measurement_runner")
        || text(value, "parallel_write_policy")
            != Some("no_shared_validation_artifact_parallel_write")
    {
        return false;
    }
    let Some(task_count) = positive_usize(value, "task_count") else {
        return false;
    };
    let Some(queue_depth) = positive_usize(value, "queue_depth") else {
        return false;
    };
    positive_usize(value, "worker_count") == Some(1) && queue_depth <= task_count
}

fn positive_usize(value: &Value, key: &str) -> Option<usize> {
    let number = value.get(key).and_then(Value::as_u64)?;
    (number > 0).then(|| usize::try_from(number).ok()).flatten()
}
