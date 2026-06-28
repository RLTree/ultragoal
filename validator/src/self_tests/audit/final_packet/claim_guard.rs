use super::support;
use serde_json::{Value, json};
use std::path::Path;

#[test]
fn final_packet_claim_guard_accepts_only_fail_closed_blocker_shape() {
    let root = crate::self_tests::boundaries::support::temp_root("final-packet-claim-guard");
    support::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":[]}),
    );
    let store = crate::schema_catalog::load(&crate::self_tests::boundaries::support::repo_root());
    let current = crate::package::inventory::package_digest(&root).expect("digest");
    let receipt = support::write_fail_closed_proof(&root, &current);

    let failures = crate::audit::final_packet::claim_guard_failures(&root, &store);
    assert!(failures.is_empty(), "{failures:?}");

    assert_guard_failure(
        &root,
        &store,
        mutate(&receipt, |value| {
            value["target_revision"]["value"] =
                json!(crate::self_tests::boundaries::support::sha('b'));
        }),
        "final_packet_proof_target_digest_mismatch",
    );
    assert_guard_failure(
        &root,
        &store,
        mutate(&receipt, |value| value["status"] = json!("pending")),
        "final_packet_proof_guard_status_not_fail",
    );
    assert_guard_failure(
        &root,
        &store,
        mutate(&receipt, |value| {
            value["claim_ceiling"] = json!("final_packet_evidence_dereferenced")
        }),
        "final_packet_proof_guard_claim_ceiling_not_blocking",
    );
    assert_guard_failure(
        &root,
        &store,
        mutate(&receipt, |value| {
            value["blocked_claim_classes"] = json!(["completion"])
        }),
        "final_packet_proof_guard_missing_blocked_claim:package_readiness",
    );
    assert_guard_failure(
        &root,
        &store,
        mutate(&receipt, |value| {
            value["failure"]["reason"] = json!("not_good_enough")
        }),
        "final_packet_proof_guard_failure_reason_missing",
    );
    assert_guard_failure(
        &root,
        &store,
        mutate(&receipt, |value| {
            value["failure"]["observed_failures"] = json!([])
        }),
        "final_packet_proof_guard_observed_failures_missing",
    );

    std::fs::remove_dir_all(root).expect("cleanup final packet claim guard");
}

#[test]
fn final_packet_claim_guard_dereferences_failed_registry_and_source_audit() {
    let root = crate::self_tests::boundaries::support::temp_root("final-packet-claim-guard-refs");
    support::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":[]}),
    );
    let store = crate::schema_catalog::load(&crate::self_tests::boundaries::support::repo_root());
    let current = crate::package::inventory::package_digest(&root).expect("digest");
    let receipt = support::write_fail_closed_proof(&root, &current);

    let mut bad_registry = receipt.clone();
    rewrite_ref(
        &root,
        &mut bad_registry,
        "registry_exposure",
        "validation_artifacts/ultragoal-audit/active-registry-exposure-current.json",
        &json!({
            "schema":"harness-ultragoal.multi-agent-registry-exposure.v1",
            "status":"fail",
            "source":"manual-json",
            "target_revision":{"kind":"package_digest","value":current},
            "claim_ceiling":"withheld_or_blocked",
            "issuer":{"tool":"ultragoal","authority":"cli_control_plane"},
            "capture_method":"fail_closed_no_capability",
            "failure":{"reason":"live_registry_reviewer_exposure_not_proven"},
            "blocked_claim_classes":["completion"]
        }),
    );
    assert_guard_failure(
        &root,
        &store,
        bad_registry,
        "final_packet_proof_registry_ref:plugin_self_law_registry_guard_wrong_source",
    );

    let mut missing_source = receipt.clone();
    missing_source
        .as_object_mut()
        .unwrap()
        .remove("source_audit");
    assert_guard_failure(
        &root,
        &store,
        missing_source,
        "final_packet_proof_ref_missing:source_audit",
    );

    let mut invalid_source = receipt.clone();
    invalid_source["source_audit"]["path"] = json!("../validator-receipt.json");
    assert_guard_failure(
        &root,
        &store,
        invalid_source,
        "final_packet_proof_ref_path_invalid:source_audit",
    );

    let mut malformed_source = receipt;
    let source_path = "validation_artifacts/ultragoal-audit/validator-receipt.json";
    std::fs::write(root.join(source_path), "{").expect("malformed source audit");
    malformed_source["source_audit"]["path"] = json!(source_path);
    assert_guard_failure(
        &root,
        &store,
        malformed_source,
        "final_packet_proof_ref_malformed:source_audit",
    );

    std::fs::remove_dir_all(root).expect("cleanup final packet claim guard refs");
}

fn mutate(value: &Value, edit: impl FnOnce(&mut Value)) -> Value {
    let mut value = value.clone();
    edit(&mut value);
    value
}

fn rewrite_ref(root: &Path, proof: &mut Value, key: &str, path: &str, value: &Value) {
    support::write_json(&root.join(path), value);
    proof[key]["path"] = json!(path);
    proof[key]["digest"] = json!(crate::digest::file(&root.join(path)).expect("ref digest"));
    proof[key]["status"] = value["status"].clone();
}

fn assert_guard_failure(
    root: &Path,
    store: &crate::schema_catalog::SchemaStore,
    value: Value,
    expected: &str,
) {
    support::write_proof(root, &value);
    let failures = crate::audit::final_packet::claim_guard_failures(root, store);
    assert!(
        failures.iter().any(|failure| failure.contains(expected)),
        "{expected}: {failures:?}"
    );
}
