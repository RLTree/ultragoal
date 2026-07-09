use super::super::full_command;
use super::fields::{json_string_array, text};
use crate::cli::live_loop::surfaces::LoopValidationSurface;
use serde_json::Value;

pub(super) fn matches_command_identity(row: &Value, surface: LoopValidationSurface) -> bool {
    if !requires_exact_cargo_identity(surface) {
        return true;
    }
    let expected_argv = full_command::product_command_argv(surface.narrow_rerun);
    text(row, "verified_local_command")
        == Some(full_command::product_command_text(surface.narrow_rerun).as_str())
        && json_string_array(row, "command_argv").as_ref() == Some(&expected_argv)
        && json_string_array(row, "verified_local_command_argv").as_ref() == Some(&expected_argv)
}

pub(super) fn matches_node_specific_result(row: &Value, surface: LoopValidationSurface) -> bool {
    if surface.id != "live_loop_measurement_rust_tests" {
        return true;
    }
    row.get("verified_local_executed_test_count")
        .and_then(Value::as_u64)
        .is_some_and(|count| count > 0)
}

pub(super) fn executed_test_count(row: &Value, surface: LoopValidationSurface) -> Option<u64> {
    if surface.id == "live_loop_measurement_rust_tests" {
        row.get("verified_local_executed_test_count")
            .and_then(Value::as_u64)
    } else {
        None
    }
}

pub(super) fn requires_exact_cargo_identity(surface: LoopValidationSurface) -> bool {
    matches!(
        surface.id,
        "build_check" | "live_loop_measurement_rust_tests"
    )
}
