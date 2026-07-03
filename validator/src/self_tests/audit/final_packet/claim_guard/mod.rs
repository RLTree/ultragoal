use super::receipt_fixtures;
use serde_json::{Value, json};
use std::path::Path;

mod source_audit;

#[test]
fn final_packet_claim_guard_accepts_only_fail_closed_blocker_shape() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("final-packet-claim-guard");
    receipt_fixtures::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":[]}),
    );
    let store = crate::schema_catalog::load(
        &crate::self_tests::boundaries::workspace_fixtures::repo_root(),
    );
    let current = crate::package::inventory::package_digest(&root).expect("digest");
    let receipt = receipt_fixtures::write_fail_closed_proof(&root, &current);

    let failures = crate::audit::final_packet::claim_guard_failures(&root, &store);
    assert!(failures.is_empty(), "{failures:?}");

    assert_guard_failure(
        &root,
        &store,
        mutate(&receipt, |value| {
            value["target_revision"]["value"] =
                json!(crate::self_tests::boundaries::workspace_fixtures::sha('b'));
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
    assert_guard_failure(
        &root,
        &store,
        mutate(&receipt, |value| {
            value["packet"]["digest"] = json!(crate::digest::ZERO)
        }),
        "final_packet_proof_packet_zero_digest_anchor",
    );
    assert_guard_failure(
        &root,
        &store,
        mutate(&receipt, |value| {
            value["packet"]["exists"] = json!(false);
            value["packet"]["digest"] =
                json!(crate::self_tests::boundaries::workspace_fixtures::sha('c'));
        }),
        "final_packet_proof_packet_absent_digest_not_null",
    );

    std::fs::remove_dir_all(root).expect("cleanup final packet claim guard");
}

#[test]
fn final_packet_claim_guard_pass_receipt_routes_to_full_value_verifier() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "final-packet-claim-guard-pass",
    );
    receipt_fixtures::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":[]}),
    );
    let store = crate::schema_catalog::load(
        &crate::self_tests::boundaries::workspace_fixtures::repo_root(),
    );
    let current = crate::package::inventory::package_digest(&root).expect("digest");
    let receipt = receipt_fixtures::write_green_proof(&root, &current);

    let failures = crate::audit::final_packet::value_claim_guard_failures(&root, &store, &receipt);
    assert!(failures.is_empty(), "{failures:?}");
    std::fs::remove_dir_all(root).expect("cleanup final packet claim guard pass");
}

fn mutate(value: &Value, edit: impl FnOnce(&mut Value)) -> Value {
    let mut value = value.clone();
    edit(&mut value);
    value
}

fn assert_guard_failure(
    root: &Path,
    store: &crate::schema_catalog::SchemaStore,
    value: Value,
    expected: &str,
) {
    receipt_fixtures::write_proof(root, &value);
    let failures = crate::audit::final_packet::claim_guard_failures(root, store);
    assert!(
        failures.iter().any(|failure| failure.contains(expected)),
        "{expected}: {failures:?}"
    );
}
