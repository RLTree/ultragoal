use super::PerformanceCommand;
use super::types::{
    BudgetClass, PERFORMANCE_BUDGET_VERSION, PERFORMANCE_COMMANDS, PERFORMANCE_RECEIPT_SCHEMA,
    PerformanceOperation,
};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

mod speed_claim_failure;
mod speed_nodes;

pub(crate) fn receipt(
    root: &Path,
    command: &PerformanceCommand,
    wall_ms: u64,
) -> Result<Value, String> {
    let candidate = crate::package::inventory::package_digest(root)?;
    let mut value = json!({
        "schema": PERFORMANCE_RECEIPT_SCHEMA,
        "schema_version": "v1",
        "issuer": {"tool": "ultragoal", "authority": "cli_control_plane"},
        "generated_at": crate::audit::clock::now_iso(),
        "root": ".",
        "command": command_value(command),
        "budget": budget_value(command.class),
        "digests": digest_value(root, &candidate)?,
        "input_size_metrics": input_metrics(root),
        "output_size_metrics": {"receipt_count_written": if command.receipt.is_some() { 1 } else { 0 }},
        "cache": cache_value(&candidate, command),
        "concurrency": concurrency_value(),
        "telemetry": telemetry_value(wall_ms),
        "speed_proof": speed_nodes::speed_proof_value(
            root,
            &candidate,
            command.operation == PerformanceOperation::Prove,
        ),
        "external_probe_policy": external_policy(command.class),
        "commands": PERFORMANCE_COMMANDS
    });
    apply_status(&mut value, command.class, wall_ms);
    Ok(value)
}

pub(crate) fn apply_status(value: &mut Value, class: BudgetClass, wall_ms: u64) {
    value["telemetry"]["wall_clock_ms"] = json!(wall_ms);
    if value.pointer("/command/name").and_then(Value::as_str) == Some("performance_budgets") {
        value["status"] = json!("pass");
        value["claim_ceiling"] = json!("budget_catalog_observation_only");
        value["performance_regression"] = regression_value("not_a_speed_claim");
        value["exit_code"] = json!(0);
        value["supported_claim_classes"] = json!([]);
        value["blocked_claim_classes"] = json!([]);
        value["failure"] = Value::Null;
        return;
    }

    let speed_proof_ready = super::receipt::speed_proof_claim_ready(value, None);
    if wall_ms <= class.cold_p95_ms() && speed_proof_ready {
        value["status"] = json!("pass");
        value["claim_ceiling"] = json!(super::receipt::SPEED_NODE_CLAIM_CEILING);
        value["performance_regression"] = regression_value("pass");
        value["exit_code"] = json!(0);
        value["supported_claim_classes"] = json!(["routine_usability"]);
        value["blocked_claim_classes"] = json!([]);
        value["failure"] = Value::Null;
    } else {
        let has_speed_nodes = value
            .pointer("/speed_proof/nodes")
            .and_then(Value::as_array)
            .is_some_and(|nodes| !nodes.is_empty());
        let (failure_id, failed_invariant, observed_value, expected_value) = if speed_proof_ready {
            (
                "cli_performance_budget_exceeded",
                "current same-candidate performance proof must finish within the declared budget",
                format!("wall_clock_ms={wall_ms}"),
                format!("wall_clock_ms<={}", class.cold_p95_ms()),
            )
        } else if has_speed_nodes {
            (
                "cli_performance_node_speed_proof_failed",
                "current same-candidate speed proof nodes must satisfy node timing and reuse laws",
                speed_claim_failure::observed_value(value),
                "each node has timing_status=pass, failure_class=none, product-work evidence, and verified telemetry reconciliation".to_string(),
            )
        } else {
            (
                "cli_performance_missing_node_speed_proof",
                "speed claims require executed work or verified same-candidate cache replay per node",
                speed_claim_failure::observed_value(value),
                "each node records proof_kind=executed or proof_kind=verified_cache_hit with product-work evidence".to_string(),
            )
        };
        value["status"] = json!("fail");
        value["claim_ceiling"] = json!("withheld_or_blocked");
        value["performance_regression"] =
            regression_value("fail_closed_until_current_baseline_receipt_passes");
        value["exit_code"] = json!(1);
        value["supported_claim_classes"] = json!([]);
        value["blocked_claim_classes"] = json!([
            "completion",
            "routine_usability",
            "product_readiness",
            "package_readiness",
            "release_readiness",
            "update_goal_eligibility"
        ]);
        value["failure"] = json!({
            "id": failure_id,
            "law_id": "cli-performance-latency-speed-iteration-fitness",
            "gate_id": "89.22",
            "check_id": "cli-performance-latency-speed-iteration-fitness",
            "failed_invariant": failed_invariant,
            "observed_value": observed_value,
            "expected_value": expected_value,
            "claim_ceiling_impact": "routine_usability_product_readiness_release_update_goal_withheld",
            "repair_class": "deterministic_enforcement",
            "severity": "hard_blocker"
        });
    }
}

pub(crate) fn digest_or_zero(root: &Path, rel: &str) -> String {
    crate::digest::file(&root.join(rel)).unwrap_or_else(|_| crate::digest::ZERO.to_string())
}

pub(crate) fn cli_binary_digest_for_path(path: Option<PathBuf>) -> String {
    path.and_then(|path| crate::digest::file(&path).ok())
        .unwrap_or_else(|| crate::digest::ZERO.to_string())
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
        "target_ms": class.cold_p95_ms(),
        "hard_ceiling_ms": class.hard_ceiling_ms(),
        "threshold_ms": class.cold_p95_ms()
    })
}

fn digest_value(root: &Path, candidate: &str) -> Result<Value, String> {
    Ok(json!({
        "source": candidate,
        "candidate": candidate,
        "cli_binary": cli_binary_digest(),
        "schema_catalog": digest(root, "schemas/schema-catalog.json")?,
        "law_graph": digest(root, "docs/mandatory-law-surfaces.json")?,
        "standards": digest(root, "templates/agent-standards/enforcement.json")?,
        "fixture_catalog": digest(root, "templates/RED_FIXTURES.json")?,
        "source_obligation": digest(root, "docs/source-obligation-matrix.json")?,
        "config": digest_or_zero(root, "plugin-manifest-draft.json")
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
    let worker_count = crate::scheduler::SchedulerConfig::default_jobs();
    json!({
        "level": worker_count,
        "worker_count": worker_count,
        "queue_depth": 0,
        "isolation_namespace": "scheduler_default_available_parallelism_minus_one_no_shared_artifact_writes"
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

fn regression_value(status: &str) -> Value {
    json!({
        "baseline_version": PERFORMANCE_BUDGET_VERSION,
        "baseline_machine_class": "local_macos_codex_desktop",
        "tolerance_percent": 0,
        "status": status
    })
}

fn digest(root: &Path, rel: &str) -> Result<String, String> {
    crate::digest::file(&root.join(rel))
}

fn cli_binary_digest() -> String {
    cli_binary_digest_for_path(std::env::current_exe().ok())
}
