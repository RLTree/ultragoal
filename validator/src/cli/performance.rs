use crate::cli::performance::types::{
    BudgetClass, PERFORMANCE_BUDGET_VERSION, PERFORMANCE_COMMANDS, PERFORMANCE_RECEIPT_SCHEMA,
    PerformanceOperation,
};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::time::Instant;

#[derive(Debug)]
pub(crate) struct PerformanceCommand {
    pub(crate) operation: PerformanceOperation,
    pub(crate) receipt: Option<PathBuf>,
    pub(crate) class: BudgetClass,
}

pub(crate) fn parse(raw: &[String]) -> Result<Option<PerformanceCommand>, String> {
    let operation = match raw {
        [a, b, ..] if a == "performance" && b == "prove" => PerformanceOperation::Prove,
        [a, b, ..] if a == "performance" && b == "verify" => PerformanceOperation::Verify,
        [a, b, ..] if a == "performance" && b == "budgets" => PerformanceOperation::Budgets,
        [a, b, c, ..] if a == "self" && b == "performance" && c == "prove" => {
            PerformanceOperation::SelfProve
        }
        _ => return Ok(None),
    };
    let class = match opt_string(raw, "--class") {
        Some(raw) => BudgetClass::from_str(&raw)
            .ok_or_else(|| "invalid --class performance budget".to_string())?,
        None => default_class(operation),
    };
    Ok(Some(PerformanceCommand {
        operation,
        receipt: opt_path(raw, "--receipt"),
        class,
    }))
}

pub(crate) fn run(root: &Path, command: &PerformanceCommand) -> Result<i32, String> {
    let start = Instant::now();
    let receipt = receipt(root, command, start.elapsed().as_millis() as u64)?;
    if let Some(path) = &command.receipt {
        crate::json_boundary::write_json(path, &receipt)?;
        println!(
            "ultragoal-performance {} operation={} class={} receipt={}",
            receipt["status"],
            command.operation.id(),
            command.class.id(),
            path.display()
        );
    } else {
        println!("{receipt}");
    }
    Ok(1)
}

pub(crate) fn receipt(
    root: &Path,
    command: &PerformanceCommand,
    wall_ms: u64,
) -> Result<Value, String> {
    let candidate = crate::package::inventory::package_digest(root)?;
    Ok(json!({
        "schema": PERFORMANCE_RECEIPT_SCHEMA,
        "schema_version": "v1",
        "issuer": {"tool": "ultragoal", "authority": "cli_control_plane"},
        "generated_at": crate::audit::clock::now_iso(),
        "root": ".",
        "status": "fail",
        "claim_ceiling": "withheld_or_blocked",
        "command": command_value(command),
        "budget": budget_value(command.class),
        "digests": digest_value(root, &candidate)?,
        "input_size_metrics": input_metrics(root),
        "output_size_metrics": {"receipt_count_written": if command.receipt.is_some() { 1 } else { 0 }},
        "cache": cache_value(&candidate, command),
        "concurrency": concurrency_value(),
        "telemetry": telemetry_value(wall_ms),
        "external_probe_policy": external_policy(command.class),
        "performance_regression": regression_value(),
        "exit_code": 1,
        "supported_claim_classes": [],
        "blocked_claim_classes": [
            "completion",
            "routine_usability",
            "product_readiness",
            "package_readiness",
            "release_readiness",
            "update_goal_eligibility"
        ],
        "failure": {
            "id": "cli_performance_not_self_hosted_final_proof",
            "law_id": "cli-performance-latency-speed-iteration-fitness",
            "gate_id": "89.22",
            "check_id": "cli-performance-latency-speed-iteration-fitness",
            "failed_invariant": "current same-candidate performance proof must pass before performance-dependent claims can emit",
            "observed_value": "transition_only_performance_receipt",
            "expected_value": "same_candidate_performance_receipt_pass_with_no_cache_and_regression_proof",
            "claim_ceiling_impact": "routine_usability_product_readiness_release_update_goal_withheld",
            "repair_class": "deterministic_enforcement",
            "severity": "hard_blocker"
        },
        "commands": PERFORMANCE_COMMANDS
    }))
}

fn command_value(command: &PerformanceCommand) -> Value {
    json!({
        "name": command.operation.id(),
        "argv": std::env::args().collect::<Vec<_>>(),
        "class": command.class.id(),
        "evidence_affecting": true
    })
}

fn budget_value(class: BudgetClass) -> Value {
    json!({
        "version": PERFORMANCE_BUDGET_VERSION,
        "class": class.id(),
        "cold_p95_ms": class.cold_p95_ms(),
        "warm_p95_ms": class.warm_p95_ms(),
        "threshold_ms": class.cold_p95_ms()
    })
}

fn digest_value(root: &Path, candidate: &str) -> Result<Value, String> {
    let cli_binary = cli_binary_digest();
    let schema_catalog = digest(root, "schemas/schema-catalog.json")?;
    let law_graph = digest(root, "docs/mandatory-law-surfaces.json")?;
    let standards = digest(root, "templates/agent-standards/enforcement.json")?;
    let fixture_catalog = digest(root, "templates/RED_FIXTURES.json")?;
    let source_obligation = digest(root, "docs/source-obligation-matrix.json")?;
    let config = digest_or_zero(root, "plugin-manifest-draft.json");
    Ok(json!({
        "source": candidate,
        "candidate": candidate,
        "cli_binary": cli_binary,
        "schema_catalog": schema_catalog,
        "law_graph": law_graph,
        "standards": standards,
        "fixture_catalog": fixture_catalog,
        "source_obligation": source_obligation,
        "config": config
    }))
}

fn input_metrics(root: &Path) -> Value {
    let manifest = crate::json_boundary::read_json(&root.join("plugin-manifest-draft.json"))
        .unwrap_or(Value::Null);
    let red = crate::json_boundary::read_json(&root.join("templates/RED_FIXTURES.json"))
        .unwrap_or(Value::Null);
    json!({
        "file_count_scanned": crate::package::inventory::inventory_paths(&manifest).len(),
        "fixture_count_executed": red.as_array().map_or(0, Vec::len),
        "receipt_count_read": 0,
        "repository_size_class": "single_plugin_proposal"
    })
}

fn cache_value(candidate: &str, command: &PerformanceCommand) -> Value {
    json!({
        "mode": "disabled",
        "key": format!("no-cache:{}:{}", candidate, command.operation.id()),
        "hits": 0,
        "misses": 0,
        "no_cache_mode_result": "executed_without_cache"
    })
}

fn concurrency_value() -> Value {
    json!({
        "level": 1,
        "worker_count": 1,
        "queue_depth": 0,
        "isolation_namespace": "single_process_no_shared_mutable_cache"
    })
}

fn telemetry_value(wall_ms: u64) -> Value {
    json!({
        "start_timestamp": crate::audit::clock::now_iso(),
        "end_timestamp": crate::audit::clock::now_iso(),
        "wall_clock_ms": wall_ms,
        "cpu_ms": null,
        "peak_memory_bytes": null,
        "io_bytes": null,
        "external_call_count": 0,
        "external_wait_ms": 0,
        "timeout_count": 0,
        "retry_count": 0
    })
}

fn external_policy(class: BudgetClass) -> Value {
    json!({
        "live_probe_class": class.id() == "external_live",
        "timeout_ms": if class.id() == "external_live" { 30_000 } else { 0 },
        "retry_backoff": if class.id() == "external_live" { "bounded_exponential_backoff" } else { "not_applicable_no_external_calls" },
        "offline_fallback_claim_behavior": "dependent_live_surface_claims_blocked"
    })
}

fn regression_value() -> Value {
    json!({
        "baseline_version": PERFORMANCE_BUDGET_VERSION,
        "baseline_machine_class": "local_macos_codex_desktop",
        "tolerance_percent": 0,
        "status": "fail_closed_until_current_baseline_receipt_passes"
    })
}

fn digest(root: &Path, rel: &str) -> Result<String, String> {
    crate::digest::file(&root.join(rel))
}

pub(crate) fn digest_or_zero(root: &Path, rel: &str) -> String {
    crate::digest::file(&root.join(rel)).unwrap_or_else(|_| crate::digest::ZERO.to_string())
}

fn cli_binary_digest() -> String {
    cli_binary_digest_for_path(std::env::current_exe().ok())
}

pub(crate) fn cli_binary_digest_for_path(path: Option<PathBuf>) -> String {
    path.and_then(|path| crate::digest::file(&path).ok())
        .unwrap_or_else(|| crate::digest::ZERO.to_string())
}

fn default_class(operation: PerformanceOperation) -> BudgetClass {
    match operation {
        PerformanceOperation::Budgets => BudgetClass::Instant,
        PerformanceOperation::Verify => BudgetClass::Focused,
        PerformanceOperation::Prove | PerformanceOperation::SelfProve => BudgetClass::StrictLocal,
    }
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
pub(crate) mod receipt;
pub(crate) mod types;
