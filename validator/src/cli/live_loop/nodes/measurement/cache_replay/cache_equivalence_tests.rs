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

#[test]
fn build_check_replay_requires_canonical_cargo_build_identity() {
    let fixture = ReplayFixture::build_check();
    write_timing_row(&fixture.root, timing_row(&fixture));
    assert!(cache_hit(&fixture, &fixture.input_digest).is_some());

    for bad_row in [
        timing_row(&fixture).with_value(
            "canonical_full_command",
            json!("cargo build --bin ultragoal --quiet"),
        ),
        timing_row(&fixture).with_value(
            "verified_local_command",
            json!("cargo build --bin ultragoal --quiet"),
        ),
        timing_row(&fixture).with_value(
            "verified_local_command_argv",
            json!(["cargo", "build", "--bin", "ultragoal", "--quiet"]),
        ),
        timing_row(&fixture).with_value(
            "command_argv",
            json!(["cargo", "build", "--bin", "ultragoal", "--quiet"]),
        ),
        timing_row(&fixture).with_value(
            "command_argv",
            json!(["cargo", "build", "--offline", "--quiet"]),
        ),
        timing_row(&fixture).with_value(
            "command_argv",
            json!(["cargo", "build", "--offline", "--bin", "ultragoal"]),
        ),
        timing_row(&fixture).with_value(
            "command_argv",
            json!([
                "cargo",
                "check",
                "--offline",
                "--bin",
                "ultragoal",
                "--quiet"
            ]),
        ),
    ] {
        write_timing_row(&fixture.root, bad_row);
        assert!(cache_hit(&fixture, &fixture.input_digest).is_none());
    }
    std::fs::remove_dir_all(fixture.root).expect("cleanup build command identity replay");
}

#[test]
fn measurement_rust_tests_replay_requires_canonical_test_identity_and_nonzero_tests() {
    let fixture = ReplayFixture::measurement_rust_tests();
    write_timing_row(&fixture.root, timing_row(&fixture));
    assert!(
        cache_hit(&fixture, &fixture.input_digest).is_none(),
        "Lane 014 replay must not accept local-only timing/cache rows without non-self-authored command authority"
    );

    for bad_row in [
        timing_row(&fixture).with_value(
            "canonical_full_command",
            json!("cargo test --offline live_loop::nodes::measurement --lib"),
        ),
        timing_row(&fixture).with_value(
            "verified_local_command",
            json!(
                "cargo test --offline live_loop::nodes::measurement|live_loop::graph --lib --quiet"
            ),
        ),
        timing_row(&fixture).with_value(
            "verified_local_command_argv",
            json!([
                "cargo",
                "test",
                "--offline",
                "live_loop::nodes::measurement|live_loop::graph",
                "--lib",
                "--quiet"
            ]),
        ),
        timing_row(&fixture).with_value(
            "command_argv",
            json!(["cargo", "test", "--offline", "--lib", "--quiet"]),
        ),
        timing_row(&fixture).with_value(
            "command_argv",
            json!([
                "cargo",
                "test",
                "--offline",
                "live_loop::nodes",
                "--lib",
                "--quiet"
            ]),
        ),
        timing_row(&fixture).with_value("verified_local_executed_test_count", json!(0)),
        timing_row(&fixture).without_key("verified_local_executed_test_count"),
    ] {
        write_timing_row(&fixture.root, bad_row);
        assert!(cache_hit(&fixture, &fixture.input_digest).is_none());
    }
    std::fs::remove_dir_all(fixture.root).expect("cleanup measurement rust test identity replay");
}

#[test]
fn measurement_rust_tests_replay_rejects_stale_and_mismatched_results() {
    let fixture = ReplayFixture::measurement_rust_tests();
    for bad_row in [
        timing_row(&fixture).with_value("candidate_digest", json!(digest("prior-candidate"))),
        timing_row(&fixture).with_value("input_digest", json!(digest("stale-test-input"))),
        timing_row(&fixture).with_value("current_input_digest", json!(digest("stale-test-input"))),
        timing_row(&fixture).with_value("result_digest", json!(digest("wrong-result"))),
        timing_row(&fixture).with_value("output_digest", json!(digest("wrong-output"))),
        timing_row(&fixture)
            .with_value(
                "result_digest",
                json!(digest("self-consistent-wrong-result")),
            )
            .with_value(
                "verified_local_result_digest",
                json!(digest("self-consistent-wrong-result")),
            ),
        timing_row(&fixture)
            .with_value("proof_kind", json!("executed"))
            .with_value("cache_hit", json!(true))
            .with_value("work_unit_count", json!(0)),
        timing_row(&fixture)
            .with_value("exit_status", json!(1))
            .with_value("validation_status", json!("fail")),
    ] {
        write_timing_row(&fixture.root, bad_row);
        assert!(cache_hit(&fixture, &fixture.input_digest).is_none());
    }
    std::fs::remove_dir_all(fixture.root).expect("cleanup stale measurement replay");
}

#[test]
fn build_check_replay_rejects_stale_input_result_and_no_execution_rows() {
    let fixture = ReplayFixture::build_check();
    for bad_row in [
        timing_row(&fixture).with_value("input_digest", json!(digest("stale-build-input"))),
        timing_row(&fixture).with_value("current_input_digest", json!(digest("stale-build-input"))),
        timing_row(&fixture).with_value("result_digest", json!(digest("wrong-result"))),
        timing_row(&fixture).with_value("output_digest", json!(digest("wrong-output"))),
        timing_row(&fixture)
            .with_value("cache_hit", json!(true))
            .with_value("proof_kind", json!("executed"))
            .with_value("work_unit_count", json!(0)),
    ] {
        write_timing_row(&fixture.root, bad_row);
        assert!(cache_hit(&fixture, &fixture.input_digest).is_none());
    }
    std::fs::remove_dir_all(fixture.root).expect("cleanup stale build replay");
}
