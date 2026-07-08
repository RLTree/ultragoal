use super::verified_local_hit;
use crate::cli::live_loop::LiveLoopCommand;
use crate::cli::live_loop::nodes::measurement::ObservationMode;
use serde_json::json;

#[path = "cache_equivalence_tests.rs"]
mod cache_equivalence_tests;
#[path = "test_rows.rs"]
mod test_rows;
use self::test_rows::*;

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
            ObservationMode::LoopRunSnapshot,
        )
        .is_none()
    );

    std::fs::remove_dir_all(fixture.root).expect("cleanup cache replay");
}

#[test]
fn cache_replay_reads_cache_records_as_validation_reuse_inputs() {
    let fixture = ReplayFixture::new();
    crate::json_boundary::write_json(
        &fixture.root.join(super::NODE_TIMING_REL),
        &json!({"cache_records": [timing_row(&fixture)]}),
    )
    .expect("cache record row");

    let replay =
        cache_hit(&fixture, &fixture.input_digest).expect("cache record replay input accepted");

    assert_eq!(replay.run.exit_code, 0);
    assert_eq!(replay.prior_result_digest, result_digest());
    std::fs::remove_dir_all(fixture.root).expect("cleanup cache record replay");
}

#[test]
fn cache_replay_falls_back_to_node_timing_when_cache_has_no_matching_record() {
    let fixture = ReplayFixture::new();
    write_validation_cache_row(
        &fixture.root,
        timing_row(&fixture).with_value("cache_key", json!(digest("stale-cache"))),
    );
    write_timing_row(&fixture.root, timing_row(&fixture));

    let replay = cache_hit(&fixture, &fixture.input_digest)
        .expect("node timing replay remains usable when compact cache misses");

    assert_eq!(replay.run.exit_code, 0);
    assert_eq!(replay.prior_result_digest, result_digest());
    std::fs::remove_dir_all(fixture.root).expect("cleanup cache miss fallback");
}

#[test]
fn cache_replay_reuses_validation_result_when_observability_is_partial() {
    let fixture = ReplayFixture::new();
    write_timing_row(
        &fixture.root,
        timing_row(&fixture)
            .with_value("timing_status", json!("partial"))
            .with_value(
                "failure_class",
                json!("live_loop_telemetry_reconciliation_missing"),
            )
            .with_value(
                "telemetry_reconciliation_status",
                json!("query_or_explain_reconciliation_failed"),
            )
            .with_value(
                "telemetry_reconciliation",
                json!({"status": "query_or_explain_reconciliation_failed"}),
            )
            .with_value("observability_status", json!("partial"))
            .with_value("speed_claim_status", json!("withheld"))
            .with_value(
                "observability_failure_class",
                json!("live_loop_observability_partial"),
            ),
    );

    let replay = cache_hit(&fixture, &fixture.input_digest)
        .expect("same-input validation cache replay survives partial telemetry");
    assert_eq!(replay.run.exit_code, 0);
    assert_eq!(
        replay.telemetry_reconciliation.status,
        "query_or_explain_reconciliation_failed"
    );
    assert!(
        cache_hit_for_observation(
            &fixture,
            &fixture.input_digest,
            ObservationMode::FullRoundtrip
        )
        .is_none(),
        "full measurement must not replay partial telemetry when the next repair asks for reconciliation"
    );
    std::fs::remove_dir_all(fixture.root).expect("cleanup partial telemetry cache replay");
}

#[test]
fn cache_replay_rejects_cached_reconciliation_without_status() {
    let fixture = ReplayFixture::new();
    write_timing_row(
        &fixture.root,
        timing_row(&fixture).with_value("telemetry_reconciliation", json!({"status": ""})),
    );

    assert!(
        cache_hit(&fixture, &fixture.input_digest).is_none(),
        "cache replay needs an explicit telemetry reconciliation status even when validation is reusable"
    );
    std::fs::remove_dir_all(fixture.root).expect("cleanup missing telemetry status");
}

#[test]
fn cache_replay_reuses_same_input_from_prior_candidate() {
    let fixture = ReplayFixture::new();
    write_timing_row(
        &fixture.root,
        timing_row(&fixture).with_value("candidate_digest", json!(digest("prior-candidate"))),
    );

    let replay = cache_hit(&fixture, &fixture.input_digest)
        .expect("same-input prior-candidate cache replay");
    assert_eq!(
        replay.invalidation_proof,
        "cache_key_current_input_digest_command_contract_runtime_model_versions_and_environment_matched"
    );
    std::fs::remove_dir_all(fixture.root).expect("cleanup prior candidate cache replay");
}

#[test]
fn cache_replay_rejects_prior_rows_without_product_equivalence() {
    let fixture = ReplayFixture::new();
    for bad_row in [
        timing_row(&fixture).with_value("cache_key", json!(digest("other-cache"))),
        timing_row(&fixture).with_value("input_digest", json!(digest("other-input"))),
        timing_row(&fixture).with_value("canonical_full_command", json!("cargo test --lib")),
        timing_row(&fixture).with_value("proof_kind", json!("planned")),
        timing_row(&fixture).with_value("exit_status", json!(1)),
        timing_row(&fixture).with_value("output_digest", json!(digest("wrong-output"))),
        timing_row(&fixture).with_value(
            "verified_local_result_digest",
            json!(digest("wrong-result")),
        ),
        timing_row(&fixture).with_value("work_unit_count", json!(0)),
        timing_row(&fixture).with_value("equivalence_status", json!("unknown")),
        timing_row(&fixture).without_key("runtime_execution_model"),
        timing_row(&fixture).with_value("command_argv", json!([])),
        timing_row(&fixture).with_value("actual_work_duration_ms", json!(0)),
        timing_row(&fixture).with_value("reconciled_command_duration_ms", json!(1)),
        timing_row(&fixture).without_key("product_latency_ms"),
        timing_row(&fixture).with_value("product_latency_ms", json!(1)),
        timing_row(&fixture).without_key("telemetry_reconciliation_duration_ms"),
        timing_row(&fixture).without_key("telemetry_reconciliation"),
        timing_row(&fixture).without_key("actual_work_duration_ms"),
    ] {
        write_timing_row(&fixture.root, bad_row);
        assert!(cache_hit(&fixture, &fixture.input_digest).is_none());
    }
    std::fs::remove_dir_all(fixture.root).expect("cleanup cache replay");
}
