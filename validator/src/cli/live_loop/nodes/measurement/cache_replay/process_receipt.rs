use super::fields::{json_string_array, text, valid_digest};
use crate::cli::live_loop::nodes::measurement::full_command;
use crate::cli::live_loop::surfaces::LoopValidationSurface;
use serde_json::Value;
use std::path::{Component, Path, PathBuf};

pub(super) fn matches(
    root: &Path,
    row: &Value,
    surface: LoopValidationSurface,
    candidate: &str,
    exit_code: i32,
    launch_error: bool,
    stdout_digest: &str,
    stderr_digest: &str,
    output_digest: &str,
    result_digest: &str,
) -> bool {
    let Some(receipt_rel) = text(row, "command_observation_receipt") else {
        return false;
    };
    let Some(expected_receipt_digest) =
        valid_digest(text(row, "command_observation_receipt_digest").unwrap_or(""))
    else {
        return false;
    };
    let Some(expected_process_digest) =
        valid_digest(text(row, "process_result_digest").unwrap_or(""))
    else {
        return false;
    };
    if expected_process_digest != result_digest {
        return false;
    }
    let Some(receipt) = read_receipt(root, receipt_rel) else {
        return false;
    };
    if crate::digest::canonical_json(&receipt) != expected_receipt_digest {
        return false;
    }
    identity_matches(&receipt, surface, candidate)
        && process_result_matches(
            &receipt,
            exit_code,
            launch_error,
            stdout_digest,
            stderr_digest,
            output_digest,
            result_digest,
        )
}

fn read_receipt(root: &Path, receipt_rel: &str) -> Option<Value> {
    let rel = PathBuf::from(receipt_rel);
    if rel.is_absolute()
        || rel
            .components()
            .any(|component| matches!(component, Component::ParentDir))
    {
        return None;
    }
    crate::json_boundary::read_json(&root.join(rel)).ok()
}

fn identity_matches(receipt: &Value, surface: LoopValidationSurface, candidate: &str) -> bool {
    let expected_argv = full_command::product_command_argv(surface.narrow_rerun);
    path_text(receipt, &["command_identity", "node_id"]) == Some(surface.id)
        && path_text(receipt, &["candidate_digest"]) == Some(candidate)
        && path_text(receipt, &["command_identity", "canonical_full_command"])
            == Some(surface.canonical_full_command)
        && path_text(receipt, &["command_identity", "verified_local_command"])
            == Some(full_command::product_command_text(surface.narrow_rerun).as_str())
        && json_string_array(
            receipt
                .get("command_identity")
                .expect("checked below by comparison"),
            "command_argv",
        )
        .as_ref()
            == Some(&expected_argv)
}

fn process_result_matches(
    receipt: &Value,
    exit_code: i32,
    launch_error: bool,
    stdout_digest: &str,
    stderr_digest: &str,
    output_digest: &str,
    result_digest: &str,
) -> bool {
    let Some(process) = receipt.get("process_result_authority") else {
        return false;
    };
    process.get("exit_status").and_then(Value::as_i64) == Some(i64::from(exit_code))
        && process.get("launch_error").and_then(Value::as_bool) == Some(launch_error)
        && path_text(process, &["stdout_digest"]) == Some(stdout_digest)
        && path_text(process, &["stderr_digest"]) == Some(stderr_digest)
        && path_text(process, &["output_digest"]) == Some(output_digest)
        && path_text(process, &["result_digest"]) == Some(result_digest)
        && path_text(process, &["redaction_status"]) == Some("pass")
        && path_text(process, &["bounded_output_status"])
            == Some("digest_only_raw_output_not_retained")
        && process
            .get("work_unit_count")
            .and_then(Value::as_u64)
            .is_some_and(|count| count > 0)
}

fn path_text<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_str()
}
