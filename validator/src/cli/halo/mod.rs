use serde_json::Value;
use std::path::{Path, PathBuf};

mod detect;
mod proof;
#[cfg(test)]
mod tests;

pub(crate) const LAW_ID: &str = "halo-ranked-harness-change-optimization";
const DEFAULT_RECEIPT: &str = "validation_artifacts/halo/capability-receipt.json";
const DEFAULT_APP: &str = "/Applications/HALO.app";
const REGISTRY: &str = "docs/halo-adapter-registry.json";
const SCHEMA: &str = "harness-ultragoal.halo-capability-receipt.v1";

#[derive(Debug)]
pub(crate) struct HaloCommand {
    receipt: PathBuf,
    app_path: PathBuf,
}

pub(crate) fn parse(raw: &[String]) -> Result<Option<HaloCommand>, String> {
    if raw.first().map(String::as_str) != Some("halo") {
        return Ok(None);
    }
    match raw {
        [_, area, action, ..] if area == "capability" && action == "prove" => {
            Ok(Some(HaloCommand {
                receipt: opt_path(raw, "--receipt")
                    .unwrap_or_else(|| PathBuf::from(DEFAULT_RECEIPT)),
                app_path: opt_path(raw, "--app-path").unwrap_or_else(|| PathBuf::from(DEFAULT_APP)),
            }))
        }
        _ => Err("unknown ultragoal halo command".to_string()),
    }
}

pub(crate) fn run(root: &Path, command: &HaloCommand) -> Result<i32, String> {
    let receipt = proof::build_receipt(root, command)?;
    let path = crate::output_path::claim_artifact_path(root, &command.receipt, "HALO receipt")?;
    crate::json_boundary::write_json(&path, &receipt)?;
    print_receipt(&command.receipt, &receipt);
    Ok(i32::from(
        receipt.get("status").and_then(Value::as_str) != Some("pass"),
    ))
}

pub(crate) fn receipt_failures(root: &Path, receipt: &Value) -> Vec<String> {
    proof::receipt_failures(root, receipt)
}

fn opt_path(args: &[String], key: &str) -> Option<PathBuf> {
    args.iter()
        .position(|arg| arg == key)
        .and_then(|index| args.get(index + 1))
        .map(PathBuf::from)
}

fn print_receipt(receipt: &Path, value: &Value) {
    let observability = value
        .get("observability_receipt")
        .and_then(Value::as_object);
    let observed = |key: &str| {
        observability
            .and_then(|object| object.get(key))
            .and_then(Value::as_str)
            .or_else(|| {
                observability
                    .and_then(|object| object.get("trace"))
                    .and_then(Value::as_object)
                    .and_then(|trace| trace.get(key))
                    .and_then(Value::as_str)
            })
            .unwrap_or("<missing>")
    };
    println!(
        "ultragoal-halo {} candidate={} receipt={} mode={} authority={} run_id={} correlation_id={} trace_id={} span_id={} failure_class={} claim_impact={}",
        value
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("fail"),
        value
            .get("candidate_digest")
            .and_then(Value::as_str)
            .unwrap_or("<missing>"),
        receipt.display(),
        value
            .get("invocation_mode")
            .and_then(Value::as_str)
            .unwrap_or("<missing>"),
        value
            .get("authority_class")
            .and_then(Value::as_str)
            .unwrap_or("<missing>"),
        observed("run_id"),
        observed("correlation_id"),
        observed("trace_id"),
        observed("span_id"),
        observed("failure_class"),
        value
            .get("claim_impact")
            .and_then(Value::as_str)
            .unwrap_or("<missing>")
    );
}
