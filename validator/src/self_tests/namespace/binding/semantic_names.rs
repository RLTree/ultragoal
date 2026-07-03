use super::{contains, write_text};
use serde_json::json;

#[test]
fn namespace_semantic_names_reject_opaque_gate_number_paths() {
    let root = crate::self_tests::boundaries::support::temp_root("namespace-gate-number-name");
    write_text(
        &root.join("validator/src/audit/research/trace/gate92.rs"),
        "pub(crate) fn marker() {}\n",
    );
    let failures = crate::audit::namespace::law::value_failures(&root, &json!({"resources":[]}));
    assert!(
        contains(
            &failures,
            "namespace_validator_source_opaque_gate_number_name"
        ),
        "{failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup namespace gate number name");
}

#[test]
fn namespace_semantic_names_reject_goal_work_path_labels() {
    let root = crate::self_tests::boundaries::support::temp_root("namespace-goal-work-label");
    for path in [
        "validator/src/cli/observe/fitting/mod.rs",
        "validator/src/cli/observe/production_proof/mod.rs",
        "validator/src/cli/live_loop/nodes.rs",
        "validator/src/cli/live_loop/proof_status.rs",
        "validator/src/claim/goal/root_phase.rs",
        "validator/src/audit/gate92/mod.rs",
        "validator/src/audit/phase4_rebind.rs",
        "validator/src/cli/progress/checkpoint.rs",
    ] {
        write_text(&root.join(path), "pub(crate) fn marker() {}\n");
    }
    let failures = crate::audit::namespace::law::value_failures(&root, &json!({"resources":[]}));
    assert!(
        contains(
            &failures,
            "namespace_validator_source_product_opaque_goal_work_label"
        ),
        "{failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup namespace goal work label");
}

#[test]
fn namespace_semantic_names_reject_goal_work_identifier_names() {
    let root = crate::self_tests::boundaries::support::temp_root("namespace-goal-work-identifier");
    write_text(
        &root.join("validator/src/cli/observe/command_roundtrip/mod.rs"),
        "pub(crate) mod helpers;\npub(crate) mod utils;\npub(crate) mod common;\npub(crate) mod shared;\npub(crate) struct ProductionProof;\npub(crate) struct ProofStatus;\npub(crate) enum CommandState { FitCommand, ProofState, }\npub(crate) const FIT_PATH: &str = \"x\";\npub(crate) const ROOT_PHASE_ERROR: &str = \"root_phase_proof_resource_packaged\";\npub(crate) const FAILURE_ID: &str = \"final_packet_proof_status_not_pass\";\npub(crate) fn fit_command(fit_path: bool) {}\npub(crate) fn fit_goal() {}\npub(crate) fn fit_slice() {}\npub(crate) fn production_proof() {}\npub(crate) fn proof_status() {}\npub(crate) fn validation_proof() {}\npub(crate) fn production_evidence() {}\npub(crate) fn phase4_rebind() {}\npub(crate) fn checkpoint_progress() {}\npub(crate) fn todo_repair() {}\npub(crate) fn run_command(production_proof: bool, proof_status: bool) {}\nlet production_proof = true;\nlet proof_status = true;\nobservability_status: bool,\n",
    );
    let failures = crate::audit::namespace::law::value_failures(&root, &json!({"resources":[]}));
    assert!(
        contains(
            &failures,
            "namespace_validator_source_product_opaque_goal_work_identifier"
        ),
        "{failures:?}"
    );
    assert!(
        contains(&failures, "namespace_validator_source_generic_identifier"),
        "{failures:?}"
    );
    assert!(
        contains(
            &failures,
            "namespace_validator_source_product_opaque_goal_work_string"
        ),
        "{failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup namespace goal work identifier");
}

#[test]
fn namespace_semantic_names_accept_observability_product_names() {
    let root = crate::self_tests::boundaries::support::temp_root("namespace-observability-green");
    write_text(
        &root.join("validator/src/cli/observe/command_roundtrip/mod.rs"),
        "pub(crate) fn reconcile_command_telemetry() {}\npub(crate) fn bind_receipt_to_telemetry() {}\npub(crate) struct CommandTelemetryRoundtrip;\npub(crate) enum CommandTelemetryState { Reconciled, }\n",
    );
    write_text(
        &root.join("validator/src/audit/observability/command_inventory/mod.rs"),
        "pub(crate) fn validate_command_inventory_rows() {}\n",
    );
    let failures = crate::audit::namespace::law::value_failures(&root, &json!({"resources":[]}));
    assert!(
        !contains(
            &failures,
            "namespace_validator_source_product_opaque_goal_work"
        ),
        "{failures:?}"
    );
    assert!(
        !contains(&failures, "namespace_validator_source_generic_identifier"),
        "{failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup namespace observability green");
}

#[test]
fn namespace_semantic_names_reject_goal_work_artifact_path_segments() {
    let root = crate::self_tests::boundaries::support::temp_root("namespace-artifact-segment");
    std::fs::create_dir_all(&root).expect("namespace artifact segment root");
    let failures = crate::audit::namespace::law::value_failures(
        &root,
        &json!({"resources":[
            "validation_artifacts/observability/fitting/package-digest.json",
            "validation_artifacts/observability/production-proof/source-audit.json"
        ]}),
    );
    assert!(
        contains(&failures, "namespace_product_opaque_goal_work_path"),
        "{failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup namespace artifact segment");
}

#[test]
fn namespace_semantic_names_reject_generic_schema_authority_names() {
    let root = crate::self_tests::boundaries::support::temp_root("namespace-schema-authority");
    std::fs::create_dir_all(&root).expect("namespace schema authority root");
    let failures = crate::audit::namespace::law::value_failures(
        &root,
        &json!({"resources":["schemas/common-defs.schema.json"]}),
    );
    assert!(
        contains(&failures, "namespace_schema_generic_authority_path"),
        "{failures:?}"
    );
    let green = crate::audit::namespace::law::value_failures(
        &root,
        &json!({"resources":["schemas/schema-authority-primitives.schema.json"]}),
    );
    assert!(
        !contains(&green, "namespace_schema_generic_authority_path"),
        "{green:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup namespace schema authority");
}

#[test]
fn namespace_semantic_names_tamper_from_product_name_to_goal_name_fails() {
    let root = crate::self_tests::boundaries::support::temp_root("namespace-tamper-name");
    write_text(
        &root.join("validator/src/cli/observe/command_roundtrip/mod.rs"),
        "pub(crate) fn command_telemetry_roundtrip() {}\n",
    );
    let green = crate::audit::namespace::law::value_failures(&root, &json!({"resources":[]}));
    assert!(
        !contains(
            &green,
            "namespace_validator_source_product_opaque_goal_work"
        ),
        "{green:?}"
    );
    write_text(
        &root.join("validator/src/cli/observe/command_roundtrip/mod.rs"),
        "pub(crate) fn fit_command() {}\n",
    );
    let tampered = crate::audit::namespace::law::value_failures(&root, &json!({"resources":[]}));
    assert!(
        contains(
            &tampered,
            "namespace_validator_source_product_opaque_goal_work_identifier"
        ),
        "{tampered:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup namespace goal work identifier");
}
