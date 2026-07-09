use super::fixtures::{command, live_loop_timing_receipt_arg};
use crate::cli::live_loop::changed_inputs::ChangedInputs;
use crate::cli::live_loop::surfaces::surface_by_id;
use serde_json::json;
use std::path::PathBuf;

#[test]
fn live_loop_measure_replays_line_caps_strict_receipt_as_routine_cache() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "live-loop-measure-line-caps-replay",
    );
    std::fs::create_dir_all(root.join("validator/src")).expect("source dir");
    std::fs::write(root.join("validator/src/lib.rs"), "pub fn ok() {}\n").expect("source");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["validator/src/lib.rs"]}),
    )
    .expect("manifest");
    let line_caps = crate::cli::line_caps::LineCapsCommand {
        receipt: PathBuf::from("validation_artifacts/observability/line-cap-check.json"),
        jobs: Some(1),
    };
    assert_eq!(
        crate::cli::line_caps::run(&root, &line_caps).expect("line caps command"),
        0
    );

    let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
    let command = command(Some("line_caps_check"), live_loop_timing_receipt_arg());
    let surface = surface_by_id("line_caps_check").expect("line caps surface");
    let inputs = ChangedInputs::collect(&root, &candidate, &command.tier, &command.cache_mode);
    let replay_store = super::super::cache_replay::ReplayStore::load(&root, &command);
    let row = super::super::measure_surface(
        &root,
        &command,
        surface,
        &candidate,
        &inputs,
        &replay_store,
        super::super::timing::receipt::affected_set_status(inputs.changed_file_count),
        super::super::ObservationMode::FullRoundtrip,
    );

    assert_eq!(row["proof_kind"], "verified_cache_hit");
    assert_eq!(row["cache_hit"], true);
    assert_eq!(row["work_unit_count"], 0);
    assert_eq!(row["validation_status"], "pass");
    assert_eq!(row["validation_cache_status"], "reusable");
    assert_eq!(row["observability_status"], "pass");
    assert!(matches!(
        row["speed_claim_status"].as_str(),
        Some("supported" | "failed")
    ));
    assert_eq!(
        row["invalidation_proof"],
        "cache_key_current_input_digest_command_contract_runtime_model_versions_and_environment_matched"
    );
    assert_eq!(row["candidate_digest"], candidate);
    assert_eq!(row["prior_result_digest"], row["result_digest"]);
    assert_eq!(row["replayed_output_digest"], row["output_digest"]);

    std::fs::remove_dir_all(root).expect("cleanup line caps replay");
}
