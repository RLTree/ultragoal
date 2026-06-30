use crate::cli::rust::types::{RUST_COMMANDS, RUST_POLICY_VERSION, RustOperation};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::time::Instant;

pub(crate) mod observations;
pub(crate) mod receipt;
pub(crate) mod types;

#[derive(Debug)]
pub(crate) struct RustCommand {
    pub(crate) operation: RustOperation,
    pub(crate) receipt: Option<PathBuf>,
}

pub(crate) fn parse(raw: &[String]) -> Result<Option<RustCommand>, String> {
    if raw.first().map(String::as_str) != Some("rust") {
        return Ok(None);
    }
    let operation = match raw.get(1).map(String::as_str) {
        Some("toolchain") if raw.get(2).map(String::as_str) == Some("verify") => {
            RustOperation::ToolchainVerify
        }
        Some("fast") => RustOperation::Fast,
        Some("standard") => RustOperation::Standard,
        Some("release") => RustOperation::Release,
        Some("clean-proof") => RustOperation::CleanProof,
        Some("watch") => RustOperation::Watch,
        Some("memory") if raw.get(2).map(String::as_str) == Some("prove") => {
            RustOperation::MemoryProve
        }
        Some("dependency") if raw.get(2).map(String::as_str) == Some("audit") => {
            RustOperation::DependencyAudit
        }
        Some("coverage") if raw.get(2).map(String::as_str) == Some("prove") => {
            RustOperation::CoverageProve
        }
        Some("workspace")
            if raw.get(2).map(String::as_str) == Some("topology")
                && raw.get(3).map(String::as_str) == Some("check") =>
        {
            RustOperation::WorkspaceTopology
        }
        _ => return Err("unknown ultragoal rust command".to_string()),
    };
    Ok(Some(RustCommand {
        operation,
        receipt: opt_path(raw, "--receipt"),
    }))
}

pub(crate) fn run(root: &Path, command: &RustCommand) -> Result<i32, String> {
    let start = Instant::now();
    let receipt = receipt(root, command, start.elapsed().as_millis() as u64)?;
    run_with_receipt_value(command, &receipt)
}

pub(crate) fn run_with_receipt_value(
    command: &RustCommand,
    receipt: &Value,
) -> Result<i32, String> {
    let status = receipt
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("fail");
    if let Some(path) = &command.receipt {
        crate::json_boundary::write_json(path, &receipt)?;
        println!(
            "ultragoal-rust {} operation={} receipt={}",
            receipt["status"],
            command.operation.id(),
            path.display()
        );
    } else {
        println!("{receipt}");
    }
    Ok(if status == "pass" { 0 } else { 1 })
}

pub(crate) fn receipt(root: &Path, command: &RustCommand, wall_ms: u64) -> Result<Value, String> {
    let observed = observations::collect(root, command.operation);
    receipt_from_observations(root, command, wall_ms, observed)
}

pub(crate) fn receipt_from_observations(
    root: &Path,
    command: &RustCommand,
    wall_ms: u64,
    observed: observations::ObservationSet,
) -> Result<Value, String> {
    let candidate = crate::package::inventory::package_digest(root)?;
    let law = command.operation.law_id();
    let status = if observed.failures.is_empty() {
        "pass"
    } else {
        "fail"
    };
    let claim_ceiling = if status == "pass" {
        "rust_devx_observation_bound"
    } else {
        "withheld_or_blocked"
    };
    Ok(json!({
        "schema": types::RUST_RECEIPT_SCHEMA,
        "schema_version": "v1",
        "issuer": {"tool": "ultragoal", "authority": "cli_control_plane"},
        "generated_at": crate::audit::clock::now_iso(),
        "root": ".",
        "status": status,
        "claim_ceiling": claim_ceiling,
        "law_ids": [law],
        "command": {
            "name": command.operation.id(),
            "argv": std::env::args()
                .map(|arg| observations::redact_private_paths(&arg))
                .collect::<Vec<_>>(),
            "proof_surface": command.operation.proof_surface(),
            "raw_tools_are_observations_only": true
        },
        "digests": digests(root, &candidate)?,
        "toolchain": toolchain(root),
        "cache": cache(command.operation),
        "resource_discipline": resource_discipline(command.operation),
        "observed_tools": observed_tools(command.operation),
        "tool_observations": observed.value,
        "observation_failures": observed.failures,
        "claim_support": {
            "supported": [command.operation.proof_surface()],
            "excluded": ["completion", "review_readiness", "release_readiness", "product_success", "update_goal_eligibility"]
        },
        "staleness_policy": {
            "invalidates_on": [
                "source_digest_change",
                "package_digest_change",
                "cargo_lock_change",
                "toolchain_change",
                "schema_catalog_change",
                "law_graph_change",
                "fixture_catalog_change",
                "command_identity_change",
                "cache_mode_change"
            ]
        },
        "telemetry": {"wall_clock_ms": wall_ms},
        "commands": RUST_COMMANDS,
        "policy_version": RUST_POLICY_VERSION
    }))
}

fn digests(root: &Path, candidate: &str) -> Result<Value, String> {
    Ok(json!({
        "candidate": candidate,
        "source": candidate,
        "rust_toolchain": digest(root, "rust-toolchain.toml")?,
        "cargo_lock": digest(root, "Cargo.lock")?,
        "cargo_workspace": digest(root, "Cargo.toml")?,
        "cargo_config": digest(root, ".cargo/config.toml")?,
        "schema_catalog": digest(root, "schemas/schema-catalog.json")?,
        "law_graph": digest(root, "docs/mandatory-law-surfaces.json")?,
        "standards": digest(root, "templates/agent-standards/enforcement.json")?,
        "source_obligation": digest(root, "docs/source-obligation-matrix.json")?,
        "fixture_catalog": digest(root, "templates/RED_FIXTURES.json")?
    }))
}

fn toolchain(root: &Path) -> Value {
    json!({
        "rustc_version": observations::command("rustc", &["--version", "--verbose"]),
        "cargo_version": observations::command("cargo", &["--version", "--verbose"]),
        "active_toolchain": observations::command("rustup", &["show", "active-toolchain"]),
        "components_required": ["clippy", "llvm-tools-preview", "rustfmt"],
        "toolchain_file_present": root.join("rust-toolchain.toml").is_file()
    })
}

fn observed_tools(operation: RustOperation) -> Value {
    let tools = match operation {
        RustOperation::DependencyAudit => vec!["cargo-deny", "cargo-audit"],
        RustOperation::CoverageProve => vec!["cargo-llvm-cov"],
        RustOperation::Standard | RustOperation::Release => vec!["cargo-nextest", "clippy"],
        RustOperation::Watch => vec!["watchexec-or-bacon"],
        _ => vec!["cargo", "rustc", "rustup"],
    };
    json!(tools)
}

fn cache(operation: RustOperation) -> Value {
    cache_with_env(
        operation,
        std::env::var("RUSTC_WRAPPER").ok(),
        std::env::var("CARGO_TARGET_DIR").ok(),
    )
}

pub(crate) fn cache_with_env(
    operation: RustOperation,
    rustc_wrapper: Option<String>,
    cargo_target_dir: Option<String>,
) -> Value {
    json!({
        "cache_mode": if operation == RustOperation::CleanProof { "isolated_no_cache" } else { "declared_local" },
        "cargo_incremental": "0",
        "rustc_wrapper": rustc_wrapper
            .map(|value| observations::redact_private_paths(&value))
            .unwrap_or_else(|| "unset".to_string()),
        "cargo_target_dir": cargo_target_dir
            .map(|value| observations::redact_private_paths(&value))
            .unwrap_or_else(|| "target".to_string()),
        "remote_cache": "none",
        "warm_cache_supports_no_cache_claim": false
    })
}

fn resource_discipline(operation: RustOperation) -> Value {
    json!({
        "bounded_resources": true,
        "spawn_and_forget_allowed": false,
        "unbounded_channels_allowed": false,
        "blind_cleanup_allowed": false,
        "tracing_gc_core_allowed": false,
        "memory_proof_surface": operation == RustOperation::MemoryProve
    })
}

fn digest(root: &Path, rel: &str) -> Result<String, String> {
    crate::digest::file(&root.join(rel))
}

fn opt_path(args: &[String], key: &str) -> Option<PathBuf> {
    args.iter()
        .position(|arg| arg == key)
        .and_then(|index| args.get(index + 1))
        .map(PathBuf::from)
}
