use super::record_fields;
use crate::cli::live_loop::surfaces::LoopValidationSurface;
use serde_json::Value;

pub(crate) fn expected(
    row: &Value,
    node_id: &str,
    surface: LoopValidationSurface,
) -> Option<(String, Vec<String>)> {
    if node_id == "live_loop_measurement_rust_tests" {
        return Some((
            product_command_text(surface.canonical_full_command),
            product_command_argv(surface.canonical_full_command),
        ));
    }
    Some((
        record_fields::text(row, "verified_local_command")?.to_string(),
        string_array(row, "command_argv")?,
    ))
}

fn product_command_text(command_text: &str) -> String {
    product_command_argv(command_text).join(" ")
}

fn product_command_argv(command_text: &str) -> Vec<String> {
    if let Some(rest) = command_text.strip_prefix("target/debug/ultragoal") {
        let mut argv = vec!["ultragoal".to_string()];
        argv.extend(split_simple_args(rest.trim_start()));
        return argv;
    }
    split_simple_args(command_text)
}

fn split_simple_args(command_text: &str) -> Vec<String> {
    command_text
        .split_whitespace()
        .map(ToString::to_string)
        .collect()
}

fn string_array(row: &Value, key: &str) -> Option<Vec<String>> {
    row.get(key).and_then(Value::as_array).map(|items| {
        items
            .iter()
            .filter_map(|item| item.as_str().map(ToString::to_string))
            .collect()
    })
}
