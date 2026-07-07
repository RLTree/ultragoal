use serde_json::json;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

#[path = "cache_hit.rs"]
mod cache_hit;

pub(super) use cache_hit::verified_cache_row;

static NEXT_TEMP_ROOT: AtomicU64 = AtomicU64::new(0);

pub(super) fn digest(ch: char) -> String {
    crate::self_tests::boundaries::workspace_fixtures::sha(ch)
}

pub(super) fn temp_root() -> std::path::PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let sequence = NEXT_TEMP_ROOT.fetch_add(1, Ordering::SeqCst);
    std::env::current_dir()
        .expect("cwd")
        .join("target")
        .join(format!(
            "ultragoal-performance-speed-nodes-{}-{stamp}-{sequence}",
            std::process::id()
        ))
}

pub(super) fn executed_row(candidate: &str) -> serde_json::Value {
    let mut row = json!({
        "node_id": "fmt_check",
        "proof_kind": "executed",
        "candidate_digest": candidate,
        "tier": "hot",
        "cache_mode": "verified-local",
        "timing_status": "pass",
        "failure_class": "none",
        "where_failed": "none",
        "why_failed": "none",
        "next_repair": "none"
    });
    insert(&mut row, "cache_hit", json!(false));
    insert(&mut row, "work_unit_count", json!(3));
    insert(&mut row, "actual_work_duration_ms", json!(1253));
    insert(&mut row, "graph_overhead_ms", json!(1));
    insert(&mut row, "result_digest", json!(digest('c')));
    insert(&mut row, "output_digest", json!(digest('d')));
    insert(&mut row, "telemetry_reconciliation_status", json!("pass"));
    insert(&mut row, "telemetry_reconciliation_duration_ms", json!(5));
    insert(&mut row, "reconciled_command_duration_ms", json!(1259));
    insert(&mut row, "product_latency_ms", json!(1254));
    insert(
        &mut row,
        "claim_name",
        json!("source-local speed node timing claim"),
    );
    insert(
        &mut row,
        "product_behavior_observed",
        json!("cargo fmt --all --check command execution"),
    );
    insert(
        &mut row,
        "proof_surface",
        json!(
            "speed node receipt with command argv, exit status, result digest, output digest, and telemetry reconciliation"
        ),
    );
    insert(
        &mut row,
        "independent_reconciliation_surface",
        json!("same-candidate telemetry and performance receipt"),
    );
    insert(
        &mut row,
        "claim_impact",
        json!("supports_live_loop_node_timing_only_no_readiness_release_completion_update_goal"),
    );
    insert(
        &mut row,
        "verified_local_command_argv",
        json!(["bash", "-lc", "cargo fmt --all --check"]),
    );
    insert(
        &mut row,
        "command_argv",
        json!(["bash", "-lc", "cargo fmt --all --check"]),
    );
    insert(&mut row, "verified_local_exit_code", json!(0));
    insert(&mut row, "exit_status", json!(0));
    insert(
        &mut row,
        "receipt_paths",
        json!(["validation_artifacts/observability/live-loop-node-timing.json"]),
    );
    insert(
        &mut row,
        "artifact_paths",
        json!(["validation_artifacts/observability/live-loop-node-timing.json"]),
    );
    insert(
        &mut row,
        "receipt_path",
        json!("validation_artifacts/observability/live-loop-node-timing.json"),
    );
    insert(
        &mut row,
        "verified_local_failure",
        json!({
            "receipt": "validation_artifacts/observability/live-loop/commands/fmt_check-command-observation.json"
        }),
    );
    insert(&mut row, "cache_key", json!(digest('e')));
    insert(&mut row, "input_digest", json!(digest('f')));
    insert(&mut row, "current_input_digest", json!(digest('f')));
    insert(&mut row, "audit_context_digest", json!(digest('g')));
    insert(
        &mut row,
        "validator_version",
        json!(crate::cli::live_loop::validator_version()),
    );
    insert(
        &mut row,
        "law_version",
        json!(crate::cli::live_loop::law_version()),
    );
    insert(
        &mut row,
        "schema_version",
        json!(crate::cli::live_loop::schema_version()),
    );
    insert(
        &mut row,
        "fixture_version",
        json!(crate::cli::live_loop::fixture_version()),
    );
    row
}

pub(super) fn bind_current_context(root: &Path, mut row: serde_json::Value) -> serde_json::Value {
    let candidate = row["candidate_digest"]
        .as_str()
        .expect("candidate")
        .to_string();
    let node_id = row["node_id"].as_str().expect("node id").to_string();
    let tier = row
        .get("tier")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("hot")
        .to_string();
    let cache_mode = row
        .get("cache_mode")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("verified-local")
        .to_string();
    let context = crate::cli::live_loop::validation_surface_context(
        root,
        &candidate,
        &node_id,
        &tier,
        &cache_mode,
    )
    .expect("known surface context");
    let input_digest = context.input_digest;
    row["tier"] = json!(tier);
    row["cache_mode"] = json!(cache_mode);
    row["audit_context_digest"] = json!(context.audit_context_digest);
    row["input_digest"] = json!(input_digest.clone());
    row["current_input_digest"] = json!(input_digest);
    row["cache_key"] = json!(context.cache_key);
    row["validator_version"] = json!(context.validator_version);
    row["law_version"] = json!(context.law_version);
    row["schema_version"] = json!(context.schema_version);
    row["fixture_version"] = json!(context.fixture_version);
    row
}

pub(super) fn write_timing_fixture(root: &std::path::Path, value: serde_json::Value) {
    std::fs::write(
        root.join(super::super::NODE_TIMING_REL),
        serde_json::to_string_pretty(&value).expect("json"),
    )
    .expect("timing");
}

pub(super) fn insert(row: &mut serde_json::Value, key: &str, value: serde_json::Value) {
    row.as_object_mut()
        .expect("speed node row object")
        .insert(key.to_string(), value);
}
