use crate::audit::artifacts;
use crate::digest;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

mod generated;
mod scheduler_execution;
pub(crate) mod speed;

pub struct ReceiptInput {
    pub root: PathBuf,
    pub red_report: PathBuf,
    pub stdout: PathBuf,
    pub stderr: PathBuf,
    pub check_ids: Vec<String>,
    pub failures: BTreeMap<String, Vec<String>>,
    pub red: BTreeMap<String, Value>,
    pub target_artifacts: Vec<Value>,
    pub start: String,
    pub status: String,
    pub validator_artifacts: Vec<Value>,
    pub command_text: String,
    pub mode: String,
    pub scheduler_metrics: Vec<crate::scheduler::Metrics>,
}

pub fn build(input: ReceiptInput) -> Result<Value, String> {
    let run_id = format!("ultragoal-audit-{}", input.start);
    let target_digest = crate::package::inventory::package_digest(&input.root)?;
    let root_identity = root_identity(&input.root);
    let generated = generated::artifacts(&input, &run_id)?;
    Ok(json!({
        "schema": "harness-ultragoal.validator-receipt.v1",
        "validator": "ultragoal-audit",
        "version": crate::audit::contract::VERSION,
        "run_id": run_id,
        "target": root_identity,
        "status": input.status,
        "commit": target_digest,
        "target_revision": {"kind": "package_digest", "value": target_digest},
        "claim_ceiling": claim_ceiling(&input.status),
        "supported_claim_classes": supported_claim_classes(&input.status),
        "blocked_claim_classes": blocked_claim_classes(),
        "blocked_claim_diagnostics": blocked_claim_diagnostics(&input.root),
        "speed_budget": speed::budget(&input.mode),
        "scheduler_execution": scheduler_execution::evidence(&input.scheduler_metrics, &target_digest),
        "root": root_identity,
        "validator_execution": execution(&input)?,
        "required_execplan_refs": required_execplan_refs(),
        "input_digests": artifacts::input_refs(&input.root)?,
        "required_check_ids": input.check_ids,
        "check_set_digest": digest::bytes(serde_json::to_string(&input.check_ids).unwrap_or_default().as_bytes()),
        "checks": checks(&input.failures),
        "required_red_fixture_ids": artifacts::safe_red_ids(&input.root),
        "red_fixture_catalog_digest": digest::file(&input.root.join("templates/RED_FIXTURES.json"))?,
        "red_fixtures": input.red,
        "generated_artifacts": generated,
        "generated_at": crate::audit::clock::now_iso()
    }))
}

fn claim_ceiling(status: &str) -> &'static str {
    if status == "pass" {
        "source_audit_pass_source_local_only"
    } else {
        "withheld_or_blocked"
    }
}

fn supported_claim_classes(status: &str) -> Vec<&'static str> {
    if status == "pass" {
        vec!["source_local_audit_checks", "red_fixture_report"]
    } else {
        Vec::new()
    }
}

fn blocked_claim_classes() -> Vec<&'static str> {
    vec![
        "completion",
        "package_readiness",
        "review_readiness",
        "release_readiness",
        "final_packet_correctness",
        "update_goal_eligibility",
        "app_registry_or_reviewer_exposure",
    ]
}

fn blocked_claim_diagnostics(root: &Path) -> Value {
    let store = crate::schema_catalog::load(root);
    let failures = crate::audit::final_packet::claim_guard_failures(root, &store);
    let path = root.join("validation_artifacts/review/final-packet-proof.json");
    json!([{
        "surface": "final_packet_proof",
        "path": "validation_artifacts/review/final-packet-proof.json",
        "digest": digest::file(&path).unwrap_or_else(|_| crate::digest::ZERO.to_string()),
        "status": if failures.is_empty() { "clear" } else { "blocked" },
        "observed_failures": failures,
        "blocked_claim_classes": blocked_claim_classes(),
        "claim_impact": "source_audit_pass_does_not_support_final_packet_registry_readiness_release_completion_or_update_goal"
    }])
}

fn required_execplan_refs() -> Vec<&'static str> {
    vec![
        "harness-ultragoal-plans-and-orchestrator-automation-hardening.md",
        "mandatory-coverage-authority-and-enforcement.md",
        "mandatory-coverage-scope-authority-and-anti-theater.md",
        "mandatory-plugin-product-cohesion-and-fit-repo-authority.md",
        "mandatory-product-fitness-quality-in-use-enforcement.md",
    ]
}

fn checks(failures: &BTreeMap<String, Vec<String>>) -> Value {
    let map = failures
        .iter()
        .map(|(id, rows)| {
            (
                id.clone(),
                json!({
                    "status": if rows.is_empty() { "pass" } else { "fail" },
                    "details": if rows.is_empty() { "pass".to_string() } else { rows.join("; ") }
                }),
            )
        })
        .collect();
    Value::Object(map)
}

fn execution(input: &ReceiptInput) -> Result<Value, String> {
    let current_exe = current_exe_result(std::env::current_exe())?;
    let exe_path = portable_path(&input.root, &current_exe, "external-validator");
    let root_identity = root_identity(&input.root);
    Ok(json!({
        "producer_actor_id": "package-author",
        "validator_actor_id": "ultragoal-rust-validator",
        "actor_disjoint": true,
        "executable_provenance": {
            "path": exe_path,
            "digest": digest::file(&current_exe)?,
            "source_artifact_set_digest": source_artifact_set_digest(&input.validator_artifacts),
            "invocation_mode": if input.command_text.contains("cargo run") { "cargo_run" } else { "direct_executable" }
        },
        "command": {
            "id": "ultragoal-audit",
            "command": input.command_text,
            "cwd": root_identity,
            "exit": if input.status == "pass" { 0 } else { 1 },
            "started_at": input.start,
            "completed_at": crate::audit::clock::now_iso()
        },
        "validator_artifacts": input.validator_artifacts,
        "input_digests": artifacts::input_refs(&input.root)?,
        "stdout": {"path": rel_path(&input.root, &input.stdout), "digest": digest::file(&input.stdout)?},
        "stderr": {"path": rel_path(&input.root, &input.stderr), "digest": digest::file(&input.stderr)?},
        "started_at": input.start,
        "completed_at": crate::audit::clock::now_iso(),
        "environment": "local Rust package audit; semantic_validation_clock=automation_tick_receipt.freshness_policy.clock_at_validation"
    }))
}

pub(crate) fn root_identity(root: &std::path::Path) -> String {
    let canonical = canonical_or_original(root)
        .to_string_lossy()
        .replace('\\', "/");
    if canonical.ends_with("/.codex/plugins/harness-ultragoal") {
        "codex-installed-plugin:harness-ultragoal".to_string()
    } else if let Some((_, version)) =
        canonical.split_once("/.codex/plugins/cache/local-harness-plugins/harness-ultragoal/")
    {
        format!("codex-plugin-cache:local-harness-plugins/harness-ultragoal/{version}")
    } else if canonical.ends_with("/harness-ultragoal-plugin-proposal") {
        "source-workspace:harness-ultragoal-plugin-proposal".to_string()
    } else {
        format!(
            "package-root:{}",
            crate::digest::bytes(canonical.as_bytes())
        )
    }
}

pub(crate) fn source_artifact_set_digest(artifacts: &[Value]) -> String {
    let mut pairs = artifacts
        .iter()
        .map(|row| {
            format!(
                "{}\0{}",
                row.get("path").and_then(Value::as_str).unwrap_or(""),
                row.get("digest").and_then(Value::as_str).unwrap_or("")
            )
        })
        .collect::<Vec<_>>();
    pairs.sort();
    digest::bytes(pairs.join("\0").as_bytes())
}

pub(super) fn rel_path(root: &std::path::Path, path: &std::path::Path) -> String {
    portable_path(root, path, "external-artifact")
}

fn portable_path(root: &std::path::Path, path: &std::path::Path, external_label: &str) -> String {
    let root = canonical_or_original(root);
    let path = canonical_or_original(path);
    path.strip_prefix(&root)
        .map(|rel| rel.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| {
            let name = path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("unknown");
            format!("<{external_label}:{name}>")
        })
}

pub(crate) fn current_exe_result(result: io::Result<PathBuf>) -> Result<PathBuf, String> {
    result.map_err(|err| format!("current executable lookup failed: {err}"))
}

pub(crate) fn generated_dir_entries(
    result: io::Result<std::fs::ReadDir>,
) -> Result<std::fs::ReadDir, String> {
    result.map_err(|err| format!("read generated dir: {err}"))
}

pub(crate) fn generated_entry_path(
    result: io::Result<std::fs::DirEntry>,
) -> Result<std::fs::DirEntry, String> {
    result.map_err(|err| format!("read generated entry: {err}"))
}

pub(crate) fn canonical_or_original(path: &Path) -> PathBuf {
    match path.canonicalize() {
        Ok(canonical) => canonical,
        Err(_) => path.to_path_buf(),
    }
}
