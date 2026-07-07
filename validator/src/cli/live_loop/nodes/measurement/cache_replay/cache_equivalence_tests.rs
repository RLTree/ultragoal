use super::*;
use serde_json::json;

#[test]
fn cache_replay_accepts_current_input_verified_cache_rows() {
    let fixture = ReplayFixture::new();
    write_timing_row(&fixture.root, verified_cache_row(&fixture));

    let replay = cache_hit(&fixture, &fixture.input_digest).expect("verified cache row replay");
    assert_eq!(replay.run.exit_code, 0);
    assert_eq!(replay.run.stdout_digest, stdout_digest());
    assert_eq!(replay.baseline.exit_code, 0);
    assert_eq!(replay.baseline.duration_ms, 200);
    assert_eq!(replay.prior_result_digest, result_digest());
    assert_eq!(replay.replayed_output_digest, output_digest());

    std::fs::remove_dir_all(fixture.root).expect("cleanup verified cache replay");
}

#[test]
fn cache_replay_rejects_verified_cache_rows_without_current_input_equivalence() {
    let fixture = ReplayFixture::new();
    for bad_row in [
        verified_cache_row(&fixture).with_value("cache_hit", json!(false)),
        verified_cache_row(&fixture).with_value("work_unit_count", json!(1)),
        verified_cache_row(&fixture).with_value("equivalence_status", json!("unknown")),
        verified_cache_row(&fixture).with_value("cache_equivalence_status", json!("miss")),
        verified_cache_row(&fixture)
            .with_value("prior_result_digest", json!(digest("wrong-result"))),
        verified_cache_row(&fixture)
            .with_value("replayed_output_digest", json!(digest("wrong-output"))),
    ] {
        write_timing_row(&fixture.root, bad_row);
        assert!(cache_hit(&fixture, &fixture.input_digest).is_none());
    }

    std::fs::remove_dir_all(fixture.root).expect("cleanup bad verified cache replay");
}
