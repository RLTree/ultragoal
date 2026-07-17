use serde_json::Value;
use std::path::{Path, PathBuf};

mod evidence;
mod proof;
mod registry;
#[cfg(test)]
mod tests;

pub(crate) const LAW_ID: &str = "harness-improvement-loop-trace-feedback-eval-codex-handoff";
const RECEIPT_SCHEMA: &str = "harness-ultragoal.improvement-loop-receipt.v1";
#[cfg(test)]
const DEFAULT_RECEIPT: &str = "validation_artifacts/improvement-loop/loop-closure-receipt.json";
const REGISTRY: &str = "docs/improvement-loop-registry.json";

#[derive(Debug)]
pub(crate) struct ImprovementLoopCommand {
    receipt: PathBuf,
}

#[cfg(test)]
pub(crate) fn parse(raw: &[String]) -> Result<Option<ImprovementLoopCommand>, String> {
    if raw.first().map(String::as_str) != Some("improvement-loop") {
        return Ok(None);
    }
    match raw {
        [_, action, ..] if action == "prove" => Ok(Some(ImprovementLoopCommand {
            receipt: opt_path(raw, "--receipt").unwrap_or_else(|| PathBuf::from(DEFAULT_RECEIPT)),
        })),
        _ => Err("unknown ultragoal improvement-loop command".to_string()),
    }
}

pub(crate) fn run(root: &Path, command: &ImprovementLoopCommand) -> Result<i32, String> {
    let receipt = proof::build_receipt(root, command)?;
    let path = crate::output_path::claim_artifact_path(
        root,
        &command.receipt,
        "improvement loop receipt",
    )?;
    crate::json_boundary::write_json(&path, &receipt)?;
    print_receipt(&command.receipt, &receipt);
    Ok(i32::from(
        receipt.get("status").and_then(Value::as_str) != Some("pass"),
    ))
}

pub(crate) fn receipt_failures(root: &Path, receipt: &Value) -> Vec<String> {
    proof::receipt_failures(root, receipt)
}

#[cfg(test)]
fn opt_path(args: &[String], key: &str) -> Option<PathBuf> {
    args.iter()
        .position(|arg| arg == key)
        .and_then(|index| args.get(index + 1))
        .map(PathBuf::from)
}

fn print_receipt(receipt: &Path, value: &Value) {
    println!(
        "ultragoal-improvement-loop {} candidate={} receipt={} run_id={} correlation_id={} claim_impact={}",
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
            .pointer("/observability_receipt/run_id")
            .and_then(Value::as_str)
            .unwrap_or("<missing>"),
        value
            .pointer("/observability_receipt/correlation_id")
            .and_then(Value::as_str)
            .unwrap_or("<missing>"),
        value
            .get("claim_impact")
            .and_then(Value::as_str)
            .unwrap_or("<missing>")
    );
}
