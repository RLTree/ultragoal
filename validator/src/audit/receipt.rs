use crate::audit::artifacts;
use crate::digest;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

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
}

pub fn build(input: ReceiptInput) -> Result<Value, String> {
    let run_id = format!("ultragoal-audit-{}", input.start);
    let target_digest = crate::package::inventory::package_digest(&input.root)?;
    let root_identity = root_identity(&input.root);
    let generated = generated_artifacts(&input, &run_id)?;
    Ok(json!({
        "schema": "harness-ultragoal.validator-receipt.v1",
        "validator": "ultragoal-audit",
        "version": crate::audit::contract::VERSION,
        "run_id": run_id,
        "target": root_identity,
        "status": input.status,
        "commit": target_digest,
        "target_revision": {"kind": "package_digest", "value": target_digest},
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

fn generated_artifacts(input: &ReceiptInput, run_id: &str) -> Result<Vec<Value>, String> {
    let mut out = vec![json!({
        "artifact_type": "red_fixture_report",
        "path": rel_path(&input.root, &input.red_report),
        "digest": digest::file(&input.red_report)?,
        "validator_run_id": run_id,
        "input_digest": digest::file(&input.root.join("templates/RED_FIXTURES.json"))?,
        "generated_at": crate::audit::clock::now_iso()
    })];
    for row in &input.target_artifacts {
        let mut item = row.clone();
        item["validator_run_id"] = json!(run_id);
        if let Some(path) = item.get("path").and_then(Value::as_str) {
            item["path"] = json!(rel_path(&input.root, &PathBuf::from(path)));
        }
        out.push(item);
    }
    out.extend(ready_for_merge_artifacts(input, run_id)?);
    Ok(out)
}

fn ready_for_merge_artifacts(input: &ReceiptInput, run_id: &str) -> Result<Vec<Value>, String> {
    let dir = input.root.join("examples/generated");
    let mut out = Vec::new();
    for entry in generated_dir_entries(std::fs::read_dir(&dir))? {
        let path = generated_entry_path(entry)?.path();
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        if !name.starts_with("READY_FOR_MERGE") || !name.ends_with(".json") {
            continue;
        }
        out.push(json!({
            "artifact_type": "ready_for_merge",
            "path": rel_path(&input.root, &path),
            "digest": digest::file(&path)?,
            "validator_run_id": run_id,
            "input_digest": digest::file(&input.root.join("fixtures/valid/minimal-goal-run.json"))?,
            "generated_at": crate::audit::clock::now_iso()
        }));
    }
    out.sort_by(|a, b| {
        a.get("path")
            .and_then(Value::as_str)
            .cmp(&b.get("path").and_then(Value::as_str))
    });
    Ok(out)
}

fn rel_path(root: &std::path::Path, path: &std::path::Path) -> String {
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
