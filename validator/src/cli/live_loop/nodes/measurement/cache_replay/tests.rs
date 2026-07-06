use super::*;
use crate::cli::live_loop::{LiveLoopAction, LiveLoopCommand, surfaces::surface_by_id};
use serde_json::json;
use std::path::{Path, PathBuf};

#[test]
fn cache_replay_accepts_only_same_input_executed_rows() {
    let fixture = ReplayFixture::new();
    let candidate = digest("candidate");
    let input_digest = digest("input");
    let cache_key = digest("cache");
    write_timing_row(
        &fixture.root,
        timing_row(fixture.surface.id, &candidate, &input_digest, &cache_key),
    );

    let replay = verified_local_hit(
        &fixture.root,
        fixture.surface,
        &candidate,
        &input_digest,
        &fixture.command,
        &cache_key,
        std::time::Instant::now(),
    )
    .expect("same-candidate cache replay");
    assert_eq!(replay.run.exit_code, 0);
    assert_eq!(replay.run.stdout_digest, stdout_digest());
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
            &candidate,
            &input_digest,
            &non_cache_command,
            &cache_key,
            std::time::Instant::now(),
        )
        .is_none()
    );

    std::fs::remove_dir_all(fixture.root).expect("cleanup cache replay");
}

#[test]
fn cache_replay_rejects_prior_rows_without_product_equivalence() {
    let fixture = ReplayFixture::new();
    let candidate = digest("candidate");
    let input_digest = digest("input");
    let cache_key = digest("cache");
    for bad_row in [
        timing_row(fixture.surface.id, &candidate, &input_digest, &cache_key)
            .with_value("candidate_digest", json!(digest("other"))),
        timing_row(fixture.surface.id, &candidate, &input_digest, &cache_key)
            .with_value("verified_local_exit_code", json!(1)),
        timing_row(fixture.surface.id, &candidate, &input_digest, &cache_key)
            .with_value("output_digest", json!(digest("wrong-output"))),
        timing_row(fixture.surface.id, &candidate, &input_digest, &cache_key).with_value(
            "verified_local_result_digest",
            json!(digest("wrong-result")),
        ),
    ] {
        write_timing_row(&fixture.root, bad_row);
        assert!(cache_hit(&fixture, &input_digest).is_none());
    }
    std::fs::remove_dir_all(fixture.root).expect("cleanup cache replay");
}

struct ReplayFixture {
    root: PathBuf,
    surface: crate::cli::live_loop::surfaces::LoopValidationSurface,
    command: LiveLoopCommand,
}

impl ReplayFixture {
    fn new() -> Self {
        let root =
            crate::self_tests::boundaries::workspace_fixtures::temp_root("live-loop-cache-replay");
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
        }
    }
}

fn cache_hit(fixture: &ReplayFixture, input_digest: &str) -> Option<CacheReplay> {
    let candidate = digest("candidate");
    let cache_key = digest("cache");
    verified_local_hit(
        &fixture.root,
        fixture.surface,
        &candidate,
        input_digest,
        &fixture.command,
        &cache_key,
        std::time::Instant::now(),
    )
}

fn timing_row(
    node_id: &str,
    candidate: &str,
    input_digest: &str,
    cache_key: &str,
) -> serde_json::Value {
    json!({
        "node_id": node_id,
        "candidate_digest": candidate,
        "tier": "hot",
        "cache_mode": "verified-local",
        "input_digest": input_digest,
        "current_input_digest": input_digest,
        "canonical_full_command": "cargo fmt --all --check",
        "proof_kind": "executed",
        "cache_key": cache_key,
        "cache_honesty": "pass",
        "telemetry_reconciliation_status": "pass",
        "verified_local_exit_code": 0,
        "verified_local_launch_error": false,
        "verified_local_stdout_digest": stdout_digest(),
        "verified_local_stderr_digest": stderr_digest(),
        "output_digest": output_digest(),
        "verified_local_output_digest": output_digest(),
        "result_digest": result_digest(),
        "verified_local_result_digest": result_digest()
    })
}

fn output_digest() -> String {
    crate::digest::bytes(
        format!("stdout={};stderr={}", stdout_digest(), stderr_digest()).as_bytes(),
    )
}

fn result_digest() -> String {
    crate::digest::bytes(format!("exit=0;launch=false;output={}", output_digest()).as_bytes())
}

fn stdout_digest() -> String {
    digest("stdout")
}

fn stderr_digest() -> String {
    digest("stderr")
}

fn digest(label: &str) -> String {
    crate::digest::bytes(label.as_bytes())
}

fn write_timing_row(root: &Path, row: serde_json::Value) {
    crate::json_boundary::write_json(&root.join(NODE_TIMING_REL), &json!({"nodes": [row]}))
        .expect("timing row");
}

trait WithValue {
    fn with_value(self, key: &str, value: serde_json::Value) -> Self;
}

impl WithValue for serde_json::Value {
    fn with_value(mut self, key: &str, value: serde_json::Value) -> Self {
        self.as_object_mut()
            .expect("timing row object")
            .insert(key.to_string(), value);
        self
    }
}
