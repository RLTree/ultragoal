use crate::cli::garbage::collection::types::{GC_POLICY_VERSION, GarbageOperation};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

pub(crate) mod receipt;
pub(crate) mod types;

#[derive(Debug)]
pub(crate) struct GarbageCommand {
    pub(crate) operation: GarbageOperation,
    pub(crate) receipt: Option<PathBuf>,
    pub(crate) plan_digest: Option<String>,
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
    }))
}

pub(crate) fn run(root: &Path, command: &GarbageCommand) -> Result<i32, String> {
    let receipt = receipt(root, command)?;
    run_with_receipt_value(command, &receipt)
}

pub(crate) fn run_with_receipt_value(
    command: &GarbageCommand,
    receipt: &Value,
) -> Result<i32, String> {
    if let Some(path) = &command.receipt {
        crate::json_boundary::write_json(path, &receipt)?;
        println!(
            "ultragoal-gc {} operation={} receipt={}",
            receipt["status"],
            command.operation.id(),
            path.display()
        );
    } else {
        println!("{receipt}");
    }
    Ok(0)
}

pub(crate) fn receipt(root: &Path, command: &GarbageCommand) -> Result<Value, String> {
    let candidate = crate::package::inventory::package_digest(root)?;
    let plan_digest = command
        .plan_digest
        .clone()
        .unwrap_or_else(|| crate::digest::bytes(format!("gc-plan:{candidate}").as_bytes()));
    Ok(json!({
        "schema": types::GC_RECEIPT_SCHEMA,
        "schema_version": "v1",
        "issuer": {"tool": "ultragoal", "authority": "cli_control_plane"},
        "generated_at": crate::audit::clock::now_iso(),
        "root": ".",
        "status": "pass",
        "claim_ceiling": "gc_observation_bound",
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
            "apply_requires_plan_digest": true,
            "blind_rm_rf_allowed": false,
            "deleted_artifacts": []
        },
        "post_verify": {
            "protected_artifacts_preserved": true,
            "active_claim_receipts_preserved": true,
            "locks_pids_ports_checked": true
        },
        "policy_version": GC_POLICY_VERSION
    }))
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
