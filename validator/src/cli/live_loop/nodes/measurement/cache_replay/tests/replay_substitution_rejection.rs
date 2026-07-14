use super::*;

#[test]
pub(crate) fn cache_replay_rejects_digest_valid_semantic_receipt_substitution() {
    let fixture = ReplayFixture::new();
    let mut row = timing_row(&fixture);
    write_timing_row(&fixture.root, row.clone());
    let receipt_path =
        command_observation_receipt_path(&fixture.root, &row).expect("materialized receipt path");
    let mut receipt = crate::json_boundary::read_json(&receipt_path).expect("read receipt");
    receipt["command_identity"]["node_id"] = json!("substituted-node");
    crate::json_boundary::write_json(&receipt_path, &receipt).expect("write substituted receipt");
    row["command_observation_receipt_digest"] = json!(crate::digest::canonical_json(&receipt));
    write_timing_row_without_receipt(&fixture.root, row);

    assert!(
        cache_hit(&fixture, &fixture.input_digest).is_none(),
        "a digest-valid receipt for different command semantics must fail closed"
    );
    std::fs::remove_dir_all(fixture.root).expect("cleanup semantic receipt substitution");
}

#[test]
pub(crate) fn cache_replay_rejects_same_input_from_wrong_candidate() {
    let fixture = ReplayFixture::build_check();
    write_timing_row(
        &fixture.root,
        timing_row(&fixture).with_value("candidate_digest", json!(digest("prior-candidate"))),
    );

    assert!(
        cache_hit(&fixture, &fixture.input_digest).is_none(),
        "candidate digest mismatch cannot satisfy verified-local replay"
    );
    std::fs::remove_dir_all(fixture.root).expect("cleanup prior candidate cache replay");
}

#[test]
pub(crate) fn cache_replay_rejects_prior_rows_without_product_equivalence() {
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
        timing_row(&fixture).without_key("surface_input_spec_status"),
        timing_row(&fixture).with_value(
            "surface_input_spec_status",
            json!("missing_surface_input_spec"),
        ),
        timing_row(&fixture).with_value("validator_authority", json!("wrong-validator-authority")),
        timing_row(&fixture).with_value("environment_class", json!("ci")),
        timing_row(&fixture).with_value("cache_class", json!("unverified_local")),
        timing_row(&fixture).with_value("claim_surface", json!("wrong-claim-surface")),
        timing_row(&fixture)
            .with_value("output_digest_expectation", json!("wrong-output-contract")),
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
