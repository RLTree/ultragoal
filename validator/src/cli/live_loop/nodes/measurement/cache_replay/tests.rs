use super::*;
use crate::cli::live_loop::{LiveLoopAction, LiveLoopCommand, surfaces::surface_by_id};
use crate::self_tests::boundaries::workspace_fixtures::temp_root;
use serde_json::json;
use std::path::{Path, PathBuf};

#[path = "cache_equivalence_tests.rs"]
mod cache_equivalence_tests;

#[test]
fn cache_replay_accepts_only_same_input_executed_rows() {
    let fixture = ReplayFixture::new();
    write_timing_row(&fixture.root, timing_row(&fixture));

    let replay = cache_hit(&fixture, &fixture.input_digest).expect("same-candidate cache replay");
    assert_eq!(replay.run.exit_code, 0);
    assert_eq!(replay.run.stdout_digest, stdout_digest());
    assert_eq!(replay.baseline.exit_code, 0);
    assert_eq!(replay.baseline.duration_ms, 200);
    assert_eq!(replay.prior_result_digest, result_digest());
    assert_eq!(replay.replayed_output_digest, output_digest());
    assert!(cache_hit(&fixture, &digest("stale-input")).is_none());

    let non_cache_command = LiveLoopCommand {
        cache_mode: "none".to_string(),
        ..fixture.command
    };
    assert!(
        verified_local_hit(
            &fixture.root,
            fixture.surface,
            &fixture.candidate,
            &fixture.input_digest,
            &non_cache_command,
            &fixture.cache_key,
            std::time::Instant::now(),
        )
        .is_none()
    );

    std::fs::remove_dir_all(fixture.root).expect("cleanup cache replay");
}

#[test]
fn cache_replay_rejects_prior_rows_without_product_equivalence() {
    let fixture = ReplayFixture::new();
    for bad_row in [
        timing_row(&fixture).with_value("candidate_digest", json!(digest("other"))),
        timing_row(&fixture).with_value("proof_kind", json!("planned")),
        timing_row(&fixture).with_value("exit_status", json!(1)),
        timing_row(&fixture).with_value("output_digest", json!(digest("wrong-output"))),
        timing_row(&fixture).with_value(
            "verified_local_result_digest",
            json!(digest("wrong-result")),
        ),
        timing_row(&fixture).with_value("work_unit_count", json!(0)),
        timing_row(&fixture).with_value("equivalence_status", json!("unknown")),
        timing_row(&fixture).with_value("command_argv", json!([])),
        timing_row(&fixture).with_value("actual_work_duration_ms", json!(0)),
        timing_row(&fixture).with_value("reconciled_command_duration_ms", json!(1)),
        timing_row(&fixture).without_key("product_latency_ms"),
        timing_row(&fixture).with_value("product_latency_ms", json!(1)),
        timing_row(&fixture).without_key("telemetry_reconciliation_duration_ms"),
        timing_row(&fixture).without_key("actual_work_duration_ms"),
    ] {
        write_timing_row(&fixture.root, bad_row);
        assert!(cache_hit(&fixture, &fixture.input_digest).is_none());
    }
    std::fs::remove_dir_all(fixture.root).expect("cleanup cache replay");
}

struct ReplayFixture {
    root: PathBuf,
    surface: crate::cli::live_loop::surfaces::LoopValidationSurface,
    command: LiveLoopCommand,
    candidate: String,
    input_digest: String,
    cache_key: String,
}

impl ReplayFixture {
    fn new() -> Self {
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

fn cache_hit(fixture: &ReplayFixture, input_digest: &str) -> Option<CacheReplay> {
    verified_local_hit(
        &fixture.root,
        fixture.surface,
        &fixture.candidate,
        input_digest,
        &fixture.command,
        &fixture.cache_key,
        std::time::Instant::now(),
    )
}

fn timing_row(fixture: &ReplayFixture) -> serde_json::Value {
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
        "validator_version": "ultragoal-rust",
        "law_version": "observability-live-loop",
        "schema_version": "harness-ultragoal.live-loop-node-timing.v1",
        "fixture_version": "source-tree-current",
        "work_unit_count": 1,
        "actual_work_duration_ms": 100,
        "graph_overhead_ms": 1,
        "equivalence_status": "executed_current_candidate_not_cache_replay",
        "invalidation_proof": "input_digest_and_candidate_checked",
        "verified_local_command": "cargo fmt --all --check",
        "verified_local_command_argv": ["bash", "-lc", "cargo fmt --all --check"],
        "command_argv": ["bash", "-lc", "cargo fmt --all --check"]
    })
    .with_value("baseline_duration_ms", json!(200))
    .with_value("baseline_exit_code", json!(0))
    .with_value("baseline_launch_error", json!(false))
    .with_value("baseline_stdout_digest", json!(stdout_digest()))
    .with_value("baseline_stderr_digest", json!(digest("stderr")))
    .with_value("baseline_failure", json!({}))
    .with_value("telemetry_reconciliation_duration_ms", json!(3))
    .with_value("reconciled_command_duration_ms", json!(104))
    .with_value("product_latency_ms", json!(104))
}

fn verified_cache_row(fixture: &ReplayFixture) -> serde_json::Value {
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

fn output_digest() -> String {
    crate::digest::bytes(
        format!("stdout={};stderr={}", stdout_digest(), digest("stderr")).as_bytes(),
    )
}

fn result_digest() -> String {
    crate::digest::bytes(format!("exit=0;launch=false;output={}", output_digest()).as_bytes())
}

fn stdout_digest() -> String {
    digest("stdout")
}

fn digest(label: &str) -> String {
    crate::digest::bytes(label.as_bytes())
}

fn write_timing_row(root: &Path, row: serde_json::Value) {
    crate::json_boundary::write_json(&root.join(NODE_TIMING_REL), &json!({"nodes": [row]}))
        .expect("timing row");
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
