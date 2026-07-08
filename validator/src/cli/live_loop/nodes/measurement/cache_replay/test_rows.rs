use super::super::{CacheReplay, NODE_TIMING_REL, VALIDATION_CACHE_REL, verified_local_hit};
use crate::cli::live_loop::nodes::measurement::ObservationMode;
use crate::cli::live_loop::{LiveLoopAction, LiveLoopCommand, surfaces::surface_by_id};
use crate::self_tests::boundaries::workspace_fixtures::temp_root;
use serde_json::json;
use std::path::{Path, PathBuf};

pub(super) struct ReplayFixture {
    pub(super) root: PathBuf,
    pub(super) surface: crate::cli::live_loop::surfaces::LoopValidationSurface,
    pub(super) command: LiveLoopCommand,
    pub(super) candidate: String,
    pub(super) input_digest: String,
    pub(super) cache_key: String,
}

impl ReplayFixture {
    pub(super) fn new() -> Self {
        let root = temp_root("live-loop-cache-replay");
        let surface = surface_by_id("fmt_check").expect("fmt surface");
        let command = LiveLoopCommand {
            action: LiveLoopAction::Measure,
            tier: "hot".to_string(),
            cache_mode: "verified-local".to_string(),
            jobs: None,
            receipt: PathBuf::from(NODE_TIMING_REL),
            node_id: Some("fmt_check".to_string()),
            measure_all: false,
        };
        Self {
            root,
            surface,
            command,
            candidate: digest("candidate"),
            input_digest: digest("input"),
            cache_key: digest("cache"),
        }
    }
}

pub(super) fn cache_hit(fixture: &ReplayFixture, input_digest: &str) -> Option<CacheReplay> {
    cache_hit_for_observation(fixture, input_digest, ObservationMode::LoopRunSnapshot)
}

pub(super) fn cache_hit_for_observation(
    fixture: &ReplayFixture,
    input_digest: &str,
    observation_mode: ObservationMode,
) -> Option<CacheReplay> {
    verified_local_hit(
        &fixture.root,
        fixture.surface,
        &fixture.candidate,
        input_digest,
        &fixture.command,
        &fixture.cache_key,
        std::time::Instant::now(),
        observation_mode,
    )
}

pub(super) fn timing_row(fixture: &ReplayFixture) -> serde_json::Value {
    json!({
        "node_id": fixture.surface.id,
        "candidate_digest": fixture.candidate,
        "tier": "hot",
        "cache_mode": "verified-local",
        "input_digest": fixture.input_digest,
        "current_input_digest": fixture.input_digest,
        "canonical_full_command": "cargo fmt --all --check",
        "proof_kind": "executed",
        "cache_hit": false,
        "cache_key": fixture.cache_key,
        "cache_honesty": "pass",
        "telemetry_reconciliation_status": "pass",
        "telemetry_reconciliation": {"status": "pass"},
        "verified_local_exit_code": 0,
        "exit_status": 0,
        "verified_local_launch_error": false,
        "receipt_paths": [NODE_TIMING_REL],
        "artifact_paths": [NODE_TIMING_REL],
        "verified_local_stdout_digest": stdout_digest(),
        "verified_local_stderr_digest": digest("stderr"),
        "output_digest": output_digest(),
        "verified_local_output_digest": output_digest(),
        "result_digest": result_digest(),
        "verified_local_result_digest": result_digest(),
        "validator_version": crate::cli::live_loop::graph::validator_version(),
        "law_version": crate::cli::live_loop::graph::law_version(),
        "schema_version": crate::cli::live_loop::graph::schema_version(),
        "fixture_version": crate::cli::live_loop::graph::fixture_version(),
        "runtime_execution_model": crate::cli::live_loop::graph::runtime_execution_model(),
        "work_unit_count": 1,
        "actual_work_duration_ms": 100,
        "graph_overhead_ms": 1,
        "equivalence_status": "executed_current_candidate_not_cache_replay",
        "invalidation_proof": "input_digest_and_candidate_checked",
        "verified_local_command": "cargo fmt --all --check",
        "verified_local_command_argv": ["cargo", "fmt", "--all", "--check"],
        "command_argv": ["cargo", "fmt", "--all", "--check"]
    })
    .with_value("validation_status", json!("pass"))
    .with_value("validation_cache_status", json!("reusable"))
    .with_value("observability_status", json!("pass"))
    .with_value("speed_claim_status", json!("supported"))
    .with_value("observability_failure_class", json!("none"))
    .with_value("baseline_duration_ms", json!(200))
    .with_value("baseline_exit_code", json!(0))
    .with_value("baseline_launch_error", json!(false))
    .with_value("baseline_stdout_digest", json!(stdout_digest()))
    .with_value("baseline_stderr_digest", json!(digest("stderr")))
    .with_value("baseline_failure", json!({}))
    .with_value("telemetry_reconciliation_duration_ms", json!(3))
    .with_value("reconciled_command_duration_ms", json!(104))
    .with_value("product_latency_ms", json!(101))
}

pub(super) fn verified_cache_row(fixture: &ReplayFixture) -> serde_json::Value {
    timing_row(fixture)
        .with_value("proof_kind", json!("verified_cache_hit"))
        .with_value("cache_hit", json!(true))
        .with_value("work_unit_count", json!(0))
        .with_value(
            "equivalence_status",
            json!("verified_same_candidate_cache_replay"),
        )
        .with_value("cache_equivalence_status", json!("pass"))
        .with_value("prior_result_digest", json!(result_digest()))
        .with_value("replayed_output_digest", json!(output_digest()))
}

pub(super) fn output_digest() -> String {
    crate::digest::bytes(
        format!("stdout={};stderr={}", stdout_digest(), digest("stderr")).as_bytes(),
    )
}

pub(super) fn result_digest() -> String {
    crate::digest::bytes(format!("exit=0;launch=false;output={}", output_digest()).as_bytes())
}

pub(super) fn stdout_digest() -> String {
    digest("stdout")
}

pub(super) fn digest(label: &str) -> String {
    crate::digest::bytes(label.as_bytes())
}

pub(super) fn write_timing_row(root: &Path, row: serde_json::Value) {
    crate::json_boundary::write_json(&root.join(NODE_TIMING_REL), &json!({"nodes": [row]}))
        .expect("timing row");
}

pub(super) fn write_validation_cache_row(root: &Path, row: serde_json::Value) {
    crate::json_boundary::write_json(&root.join(VALIDATION_CACHE_REL), &json!({"records": [row]}))
        .expect("validation cache row");
}

pub(super) trait WithValue {
    fn with_value(self, key: &str, value: serde_json::Value) -> Self;
    fn without_key(self, key: &str) -> Self;
}

impl WithValue for serde_json::Value {
    fn with_value(mut self, key: &str, value: serde_json::Value) -> Self {
        self.as_object_mut()
            .expect("timing row object")
            .insert(key.to_string(), value);
        self
    }

    fn without_key(mut self, key: &str) -> Self {
        self.as_object_mut().expect("timing row object").remove(key);
        self
    }
}
