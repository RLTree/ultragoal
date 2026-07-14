use super::*;

#[test]
pub(crate) fn cache_replay_accepts_only_same_input_executed_rows() {
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
pub(crate) fn cache_replay_reads_cache_records_as_validation_reuse_inputs() {
    let fixture = ReplayFixture::new();
    let row = timing_row(&fixture);
    write_cache_records(&fixture.root, row);

    let replay =
        cache_hit(&fixture, &fixture.input_digest).expect("cache record replay input accepted");

    assert_eq!(replay.run.exit_code, 0);
    assert_eq!(replay.prior_result_digest, result_digest());
    std::fs::remove_dir_all(fixture.root).expect("cleanup cache record replay");
}

#[test]
pub(crate) fn cache_replay_falls_back_to_node_timing_when_cache_has_no_matching_record() {
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
pub(crate) fn cache_replay_reuses_validation_result_when_observability_is_partial() {
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
pub(crate) fn cache_replay_rejects_cached_reconciliation_without_status() {
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
pub(crate) fn cache_replay_rejects_timing_row_without_process_receipt_authority() {
    let fixture = ReplayFixture::new();
    let row = timing_row(&fixture)
        .without_key("command_observation_receipt")
        .without_key("command_observation_receipt_digest")
        .without_key("process_result_digest");
    crate::json_boundary::write_json(
        &fixture.root.join(super::super::NODE_TIMING_REL),
        &json!({"nodes": [row]}),
    )
    .expect("forged timing row");

    assert!(
        cache_hit(&fixture, &fixture.input_digest).is_none(),
        "timing/cache rows cannot self-authorize without the command-result receipt"
    );
    std::fs::remove_dir_all(fixture.root).expect("cleanup missing process receipt authority");
}

#[test]
pub(crate) fn cache_replay_rejects_mismatched_process_receipt_authority() {
    let fixture = ReplayFixture::new();
    for bad_row in [
        timing_row(&fixture).with_value(
            "command_observation_receipt_digest",
            json!(digest("wrong-receipt-digest")),
        ),
        timing_row(&fixture).with_value(
            "process_result_digest",
            json!(digest("wrong-process-result")),
        ),
    ] {
        write_timing_row(&fixture.root, bad_row);
        assert!(
            cache_hit(&fixture, &fixture.input_digest).is_none(),
            "mismatched or unsafe process-result receipt authority must fail closed"
        );
    }

    let unsafe_receipt = timing_row(&fixture).with_value(
        "command_observation_receipt",
        json!("nested/../forged.json"),
    );
    crate::json_boundary::write_json(
        &fixture.root.join("forged.json"),
        &json!({"schema": "harness-ultragoal.observe-receipt.v1"}),
    )
    .expect("materialized unsafe-path bait");
    write_timing_row(&fixture.root, unsafe_receipt);
    assert!(
        cache_hit(&fixture, &fixture.input_digest).is_none(),
        "a materialized receipt addressed through a parent segment must fail closed"
    );
    std::fs::remove_dir_all(fixture.root).expect("cleanup mismatched process receipt authority");
}

#[test]
pub(crate) fn cache_replay_rejects_absent_inline_and_stale_process_receipt_authority() {
    let fixture = ReplayFixture::new();

    let absent = timing_row(&fixture);
    let absent_receipt =
        command_observation_receipt_path(&fixture.root, &absent).expect("absent receipt path");
    assert!(
        !absent_receipt.exists(),
        "fixture must begin without a receipt"
    );
    write_timing_row_without_receipt(&fixture.root, absent);
    assert!(
        !absent_receipt.exists(),
        "writing a timing row must not synthesize external command authority"
    );
    assert!(
        cache_hit(&fixture, &fixture.input_digest).is_none(),
        "a receipt path in a timing row is not evidence unless the exact receipt exists"
    );

    let inline = timing_row(&fixture).with_value(
        "command_observation_receipt",
        json!({"schema": "harness-ultragoal.observe-receipt.v1"}),
    );
    write_timing_row_without_receipt(&fixture.root, inline);
    assert!(
        cache_hit(&fixture, &fixture.input_digest).is_none(),
        "inline timing-row prose cannot replace an independently materialized receipt"
    );

    let stale = timing_row(&fixture);
    write_timing_row(&fixture.root, stale.clone());
    let receipt_path =
        command_observation_receipt_path(&fixture.root, &stale).expect("materialized receipt path");
    let mut receipt = crate::json_boundary::read_json(&receipt_path).expect("read receipt");
    receipt["process_result_authority"]["stdout_digest"] = json!(digest("mutated-stdout"));
    crate::json_boundary::write_json(&receipt_path, &receipt)
        .expect("mutate receipt after binding");
    assert!(
        cache_hit(&fixture, &fixture.input_digest).is_none(),
        "post-binding receipt mutation must invalidate replay"
    );

    std::fs::remove_dir_all(fixture.root).expect("cleanup absent inline stale authority");
}
