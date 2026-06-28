use serde_json::{Value, json};
use std::path::{Path, PathBuf};

const SCHEMA: &str = "harness-ultragoal.final-packet-proof.v1";
const PACKET: &str = "validation_artifacts/review/final-packet.json";
const CLI_PERFORMANCE: &str = "validation_artifacts/cli/performance-receipt.json";
const REGISTRY_EXPOSURE: &str =
    "validation_artifacts/ultragoal-audit/active-registry-exposure-current.json";
const SOURCE_AUDIT: &str = "validation_artifacts/ultragoal-audit/validator-receipt.json";
const COVERAGE: &str = "validation_artifacts/coverage/coverage-receipt.json";
const FIT_REPO: &str = "validation_artifacts/harness/fit-repo-receipt.json";
const PRODUCT_FITNESS: &str = "validation_artifacts/harness/product-fitness-receipt.json";
const PRODUCT_JOURNEY: &str = "validation_artifacts/harness/plugin-product-journey-receipt.json";

#[derive(Debug)]
pub(crate) struct FinalPacketCommand {
    pub(crate) receipt: PathBuf,
}

pub(crate) fn parse(raw: &[String]) -> Result<Option<FinalPacketCommand>, String> {
    match raw {
        [a, b, ..] if a == "final-packet" && b == "prove" => Ok(Some(FinalPacketCommand {
            receipt: opt_path(&raw[2..], "--receipt")?,
        })),
        [a, b, ..] if a == "packet" && b == "prove" => Ok(Some(FinalPacketCommand {
            receipt: opt_path(&raw[2..], "--receipt")?,
        })),
        _ => Ok(None),
    }
}

pub(crate) fn run(root: &Path, command: &FinalPacketCommand) -> Result<i32, String> {
    let receipt = receipt(root)?;
    crate::json_boundary::write_json(&command.receipt, &receipt)?;
    println!(
        "ultragoal-final-packet {} receipt={}",
        receipt["status"],
        command.receipt.display()
    );
    Ok(i32::from(
        receipt.get("status").and_then(Value::as_str) != Some("pass"),
    ))
}

pub(crate) fn receipt(root: &Path) -> Result<Value, String> {
    let candidate = crate::package::inventory::package_digest(root)?;
    let store = crate::schema_catalog::load(root);
    let mut value = pass_shaped(root, &candidate);
    let failures = crate::audit::final_packet::value_failures(root, &store, &value);
    if failures.is_empty() {
        return Ok(value);
    }
    value["status"] = json!("fail");
    value["claim_ceiling"] = json!("withheld_or_blocked");
    value["blocked_claim_classes"] = json!([
        "completion",
        "package_readiness",
        "review_readiness",
        "release_readiness",
        "update_goal_eligibility"
    ]);
    value["failure"] = json!({
        "reason": "final_packet_proof_not_proven",
        "observed_failures": failures
    });
    Ok(value)
}

fn pass_shaped(root: &Path, candidate: &str) -> Value {
    json!({
        "schema": SCHEMA,
        "generated_at": crate::audit::clock::now_iso(),
        "status": "pass",
        "target_revision": {"kind": "package_digest", "value": candidate},
        "packet": {"path": PACKET, "digest": digest_or_zero(root, PACKET)},
        "cli_performance": ref_row(root, CLI_PERFORMANCE, "pass"),
        "registry_exposure": ref_row(root, REGISTRY_EXPOSURE, status_or_fail(root, REGISTRY_EXPOSURE)),
        "source_audit": ref_row(root, SOURCE_AUDIT, status_or_fail(root, SOURCE_AUDIT)),
        "coverage": ref_row(root, COVERAGE, "pass"),
        "package_receipts": [
            ref_row(root, FIT_REPO, "pass"),
            ref_row(root, PRODUCT_FITNESS, "pass"),
            ref_row(root, PRODUCT_JOURNEY, "pass")
        ],
        "claim_ceiling": "final_packet_evidence_dereferenced",
        "blocked_claim_classes": [],
        "failure": Value::Null
    })
}

fn ref_row(root: &Path, rel: &str, status: &str) -> Value {
    json!({
        "path": rel,
        "digest": digest_or_zero(root, rel),
        "status": status
    })
}

fn status_or_fail(root: &Path, rel: &str) -> &'static str {
    let Ok(value) = crate::json_boundary::read_json(&root.join(rel)) else {
        return "fail";
    };
    match value.get("status").and_then(Value::as_str) {
        Some("pass") => "pass",
        _ => "fail",
    }
}

fn digest_or_zero(root: &Path, rel: &str) -> String {
    crate::digest::file(&root.join(rel)).unwrap_or_else(|_| crate::digest::ZERO.to_string())
}

fn opt_path(args: &[String], key: &str) -> Result<PathBuf, String> {
    args.iter()
        .position(|arg| arg == key)
        .and_then(|index| args.get(index + 1))
        .map(PathBuf::from)
        .ok_or_else(|| format!("missing required argument {key}"))
}
