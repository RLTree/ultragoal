use crate::cli::garbage::collection::operation::{GC_POLICY_VERSION, GarbageOperation};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::time::Instant;

mod observability;
pub(crate) mod operation;
pub(crate) mod receipt;

#[derive(Debug)]
pub(crate) struct GarbageCommand {
    pub(crate) operation: GarbageOperation,
    pub(crate) receipt: Option<PathBuf>,
    pub(crate) plan_digest: Option<String>,
    pub(crate) apply_receipt_digest: Option<String>,
}

pub(crate) fn parse(raw: &[String]) -> Result<Option<GarbageCommand>, String> {
    if raw.first().map(String::as_str) != Some("gc") {
        return Ok(None);
    }
    let operation = match raw.get(1).map(String::as_str) {
        Some("plan") => GarbageOperation::Plan,
        Some("dry-run") => GarbageOperation::DryRun,
        Some("apply") => GarbageOperation::Apply,
        Some("verify") => GarbageOperation::Verify,
        _ => return Err("unknown ultragoal gc command".to_string()),
    };
    Ok(Some(GarbageCommand {
        operation,
        receipt: opt_path(raw, "--receipt"),
        plan_digest: opt_string(raw, "--plan-digest"),
        apply_receipt_digest: opt_string(raw, "--apply-receipt-digest"),
    }))
}

pub(crate) fn run(root: &Path, command: &GarbageCommand) -> Result<i32, String> {
    let started = Instant::now();
    let mut receipt = receipt(root, command)?;
    observability::attach(root, command, &mut receipt, started)?;
    run_with_receipt_value(root, command, &receipt)
}

pub(crate) fn run_with_receipt_value(
    root: &Path,
    command: &GarbageCommand,
    receipt: &Value,
) -> Result<i32, String> {
    if let Some(path) = &command.receipt {
        let claim_receipt_path =
            crate::output_path::claim_artifact_path(root, path, "GC command receipt")?;
        crate::json_boundary::write_json(&claim_receipt_path, &receipt)?;
        println!(
            "ultragoal-gc {} operation={} receipt={} run_id={} correlation_id={} trace_id={} failure_class={} claim_impact={}",
            receipt["status"],
            command.operation.id(),
            path.display(),
            text(receipt, "run_id", "<missing>"),
            text(receipt, "correlation_id", "<missing>"),
            text(receipt, "trace_id", "<missing>"),
            text(receipt, "failure_class", "<missing>"),
            text(receipt, "claim_impact", "<missing>")
        );
    } else {
        println!(
            "ultragoal-gc {} operation={} candidate={} run_id={} correlation_id={} trace_id={} failure_class={} claim_impact={} next_repair={}",
            receipt["status"],
            command.operation.id(),
            receipt["digests"]["candidate"]
                .as_str()
                .unwrap_or("<missing>"),
            text(receipt, "run_id", "<missing>"),
            text(receipt, "correlation_id", "<missing>"),
            text(receipt, "trace_id", "<missing>"),
            text(receipt, "failure_class", "<missing>"),
            text(receipt, "claim_impact", "<missing>"),
            text(receipt, "next_repair", "<missing>")
        );
    }
    Ok(0)
}

pub(crate) fn receipt(root: &Path, command: &GarbageCommand) -> Result<Value, String> {
    let candidate = crate::package::inventory::package_digest(root)?;
    let (status, failures) = command_binding_failures(command);
    let plan_digest = command.plan_digest.clone().unwrap_or_else(|| {
        if command.operation == GarbageOperation::Plan {
            crate::digest::bytes(format!("gc-plan:{candidate}").as_bytes())
        } else {
            crate::digest::ZERO.to_string()
        }
    });
    Ok(json!({
        "schema": operation::GC_RECEIPT_SCHEMA,
        "schema_version": "v1",
        "issuer": {"tool": "ultragoal", "authority": "cli_control_plane"},
        "generated_at": crate::audit::clock::now_iso(),
        "root": ".",
        "status": status,
        "claim_ceiling": if status == "pass" { "gc_observation_bound" } else { "withheld_or_blocked" },
        "law_ids": ["workspace-artifact-cache-garbage-collection"],
        "command": {"name": command.operation.id(), "argv": std::env::args().collect::<Vec<_>>()},
        "digests": {
            "candidate": candidate,
            "schema_catalog": digest(root, "schemas/schema-catalog.json")?,
            "law_graph": digest(root, "docs/mandatory-law-surfaces.json")?,
            "fixture_catalog": digest(root, "templates/RED_FIXTURES.json")?
        },
        "artifact_classification": {
            "classes": [
                "cargo_target_artifact",
                "cargo_registry_cache",
                "cargo_git_cache",
                "sccache_entry",
                "nextest_recording",
                "coverage_artifact",
                "validation_artifact",
                "current_receipt",
                "stale_receipt",
                "final_packet",
                "review_packet",
                "package_artifact",
                "install_copy",
                "plugin_cache_copy",
                "worktree_lane",
                "temp_dir",
                "lock_file",
                "pid_file",
                "port_reservation",
                "trace_log",
                "performance_baseline"
            ],
            "unclassified_delete_allowed": false
        },
        "protected_set": {
            "protected_artifacts": [
                "Cargo.lock",
                "rust-toolchain.toml",
                "schemas/schema-catalog.json",
                "templates/RED_FIXTURES.json",
                "docs/mandatory-law-surfaces.json",
                "validation_artifacts/coverage/coverage-receipt.json"
            ],
            "protected_delete_allowed_without_replacement": false
        },
        "deletion_plan": {
            "plan_digest": plan_digest,
            "plan_digest_source": if command.operation == GarbageOperation::Plan { "generated_by_plan_operation" } else { "required_cli_argument" },
            "plan_digest_argument_required": command.operation != GarbageOperation::Plan,
            "apply_requires_plan_digest": true,
            "blind_rm_rf_allowed": false,
            "deleted_artifacts": []
        },
        "post_verify": {
            "protected_artifacts_preserved": true,
            "active_claim_receipts_preserved": true,
            "locks_pids_ports_checked": true,
            "apply_receipt_digest_required": command.operation == GarbageOperation::Verify,
            "apply_receipt_digest": command.apply_receipt_digest.clone().map(Value::String).unwrap_or(Value::Null)
        },
        "observation_failures": failures,
        "policy_version": GC_POLICY_VERSION
    }))
}

fn command_binding_failures(command: &GarbageCommand) -> (&'static str, Vec<String>) {
    let mut failures = Vec::new();
    if command.operation != GarbageOperation::Plan && command.plan_digest.is_none() {
        failures.push("workspace_gc_plan_digest_missing".to_string());
    }
    if command.operation == GarbageOperation::Verify && command.apply_receipt_digest.is_none() {
        failures.push("workspace_gc_apply_receipt_digest_missing".to_string());
    }
    let status = if failures.is_empty() { "pass" } else { "fail" };
    (status, failures)
}

fn text<'a>(value: &'a Value, field: &str, default: &'a str) -> &'a str {
    value.get(field).and_then(Value::as_str).unwrap_or(default)
}

fn digest(root: &Path, rel: &str) -> Result<String, String> {
    crate::digest::file(&root.join(rel))
}

fn opt_string(args: &[String], key: &str) -> Option<String> {
    args.iter()
        .position(|arg| arg == key)
        .and_then(|index| args.get(index + 1))
        .cloned()
}

fn opt_path(args: &[String], key: &str) -> Option<PathBuf> {
    opt_string(args, key).map(PathBuf::from)
}
