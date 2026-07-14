use super::*;

pub(crate) const COMMAND_OBSERVATION_REL: &str =
    "validation_artifacts/observability/changed-files.json";

#[test]
pub(crate) fn live_loop_measure_replays_current_input_cache_row_into_timing_output() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("cache-replay");
    std::fs::create_dir_all(&root).expect("root");
    std::process::Command::new("git")
        .arg("init")
        .current_dir(&root)
        .output()
        .expect("git init");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["plugin-manifest-draft.json"]}),
    )
    .expect("manifest");

    let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
    let command = command(Some("changed_files"), live_loop_timing_receipt_arg());
    let surface = surface_by_id("changed_files").expect("changed files surface");
    let inputs = ChangedInputs::collect(&root, &candidate, &command.tier, &command.cache_mode);
    let input_digest = crate::cli::live_loop::graph::surface_input_digest(
        surface,
        &candidate,
        inputs.surface_digest(surface),
        &inputs.audit_context_digest,
    );
    let cache_key = crate::cli::live_loop::graph::verified_local_cache_key(
        surface,
        &input_digest,
        &command.tier,
        &command.cache_mode,
    );
    let receipt = live_loop_timing_receipt_path(&root);
    std::fs::create_dir_all(receipt.parent().expect("receipt parent")).expect("receipt dir");
    let cache_row = cache_row(&root, surface, &candidate, &input_digest, &cache_key);
    crate::json_boundary::write_json(&receipt, &json!({"nodes": [cache_row.clone()]}))
        .expect("prior timing row");
    let replay_store =
        crate::cli::live_loop::nodes::measurement::cache_replay::ReplayStore::load(&root, &command);

    let row = crate::cli::live_loop::nodes::measurement::measure_surface(
        &root,
        &command,
        surface,
        &candidate,
        &inputs,
        &replay_store,
        crate::cli::live_loop::nodes::measurement::timing::receipt::affected_set_status(
            inputs.changed_file_count,
        ),
        crate::cli::live_loop::nodes::measurement::ObservationMode::FullRoundtrip,
    );

    assert_eq!(row["proof_kind"], "verified_cache_hit");
    assert_eq!(row["cache_hit"], true);
    assert_eq!(row["work_unit_count"], 0);
    assert_eq!(row["baseline_proof_kind"], "verified_baseline_reuse");
    assert_eq!(
        row["baseline_invalidation_proof"],
        "baseline_reused_from_verified_current_input_timing_row"
    );
    assert_eq!(
        row["equivalence_status"],
        "verified_same_candidate_cache_replay"
    );
    assert_eq!(
        row["invalidation_proof"],
        "cache_key_current_input_digest_command_contract_runtime_model_versions_and_environment_matched"
    );
    assert_eq!(row["prior_result_digest"], result_digest());
    assert_eq!(row["replayed_output_digest"], output_digest());
    assert_eq!(row["cache_equivalence_status"], "pass");
    assert_eq!(row["candidate_digest"], candidate);
    assert_eq!(row["input_digest"], input_digest);
    assert_eq!(row["cache_key"], cache_key);

    let mut substituted_receipt =
        crate::json_boundary::read_json(&root.join(COMMAND_OBSERVATION_REL))
            .expect("bound command observation receipt");
    substituted_receipt["command_identity"]["node_id"] = json!("substituted-node");
    crate::json_boundary::write_json(&root.join(COMMAND_OBSERVATION_REL), &substituted_receipt)
        .expect("substituted command observation receipt");
    let substituted_digest = crate::digest::canonical_json(&substituted_receipt);
    let mut substituted_row = cache_row;
    substituted_row["command_observation_receipt_digest"] = json!(substituted_digest);
    substituted_row["telemetry_reconciliation"]["command_observation_receipt_digest"] =
        substituted_row["command_observation_receipt_digest"].clone();
    crate::json_boundary::write_json(&receipt, &json!({"nodes": [substituted_row]}))
        .expect("digest-bound semantic substitution row");
    let substituted_store =
        crate::cli::live_loop::nodes::measurement::cache_replay::ReplayStore::load(&root, &command);
    let result = crate::cli::live_loop::nodes::measurement::measure_surface(
        &root,
        &command,
        surface,
        &candidate,
        &inputs,
        &substituted_store,
        crate::cli::live_loop::nodes::measurement::timing::receipt::affected_set_status(
            inputs.changed_file_count,
        ),
        crate::cli::live_loop::nodes::measurement::ObservationMode::LoopRunSnapshot,
    );
    assert_eq!(result["proof_kind"], "executed");
    assert_eq!(result["cache_hit"], false);
    assert_eq!(result["work_unit_count"], 1);
    assert_eq!(
        result["invalidation_proof"],
        "cache_not_used_current_command_executed"
    );

    std::fs::remove_dir_all(root).expect("cleanup measure cache replay");
}
