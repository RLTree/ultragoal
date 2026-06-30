use serde_json::Value;
use std::path::{Path, PathBuf};

mod proof;
mod registry;
#[cfg(test)]
mod tests;

pub(crate) const LAW_ID: &str = "promptfoo-eval-red-team-provider-separation";
pub(super) const RECEIPT_SCHEMA: &str = "harness-ultragoal.promptfoo-adapter-receipt.v1";
pub(super) const DEFAULT_RECEIPT: &str = "validation_artifacts/promptfoo/adapter-receipt.json";
pub(super) const PACKAGE_JSON: &str = "package.json";
pub(super) const PNPM_LOCK: &str = "pnpm-lock.yaml";
pub(super) const PNPM_WORKSPACE: &str = "pnpm-workspace.yaml";
pub(super) const PROVIDER_REGISTRY: &str = "docs/promptfoo-provider-registry.json";
pub(super) const SUITE_REGISTRY: &str = "docs/promptfoo-suite-registry.json";
pub(super) const PROMPTFOO_VERSION: &str = "0.121.17";

#[derive(Debug)]
pub(crate) struct PromptfooCommand {
    receipt: PathBuf,
    promptfoo_bin: PathBuf,
}

pub(crate) fn parse(raw: &[String]) -> Result<Option<PromptfooCommand>, String> {
    if raw.first().map(String::as_str) != Some("promptfoo") {
        return Ok(None);
    }
    match raw {
        [_, action, ..] if action == "prove" => Ok(Some(PromptfooCommand {
            receipt: opt_path(raw, "--receipt").unwrap_or_else(|| PathBuf::from(DEFAULT_RECEIPT)),
            promptfoo_bin: opt_path(raw, "--promptfoo-bin")
                .unwrap_or_else(|| PathBuf::from("node_modules/.bin/promptfoo")),
        })),
        _ => Err("unknown ultragoal promptfoo command".to_string()),
    }
}

pub(crate) fn run(root: &Path, command: &PromptfooCommand) -> Result<i32, String> {
    let receipt = proof::build_receipt(root, command)?;
    crate::json_boundary::write_json(&resolve(root, &command.receipt), &receipt)?;
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

fn resolve(root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}

fn print_receipt(receipt: &Path, value: &Value) {
    println!(
        "ultragoal-promptfoo {} candidate={} receipt={} run_id={} correlation_id={} claim_impact={}",
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
