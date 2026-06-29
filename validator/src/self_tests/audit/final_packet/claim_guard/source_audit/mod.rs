use super::super::support;
use serde_json::{Value, json};
use std::path::Path;

mod cases;
mod registry;

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

    registry::assert_bad_registry(&root, &store, &receipt, &current);

    let receipt = support::write_fail_closed_proof(&root, &current);
    let failed_source = assert_fail_closed_source_audit_ref(&root, &store, &receipt, &current);
    assert_source_audit_pass_self_write_is_not_circular_failure(&root, &store, &receipt, &current);
    assert_source_audit_self_rewrite_is_not_circular_failure(
        &root,
        &store,
        &failed_source,
        &current,
    );
    assert_source_audit_guard_ref_boundary_failures(&root, &store, &failed_source);
    assert_weak_source_audit_failures(&root, &store, &receipt, &current);
    registry::assert_missing_registry_failure(&root, &store, &receipt);
    assert_source_audit_status_failures(&root, &store, &receipt, &current);
    cases::assert_stale_and_malformed_source_failures(
        &root,
        &store,
        receipt,
        failed_source,
        &current,
    );

    std::fs::remove_dir_all(root).expect("cleanup final packet claim guard refs");
}

fn assert_fail_closed_source_audit_ref(
    root: &Path,
    store: &crate::schema_catalog::SchemaStore,
    receipt: &Value,
    current: &str,
) -> Value {
    let mut failed_source = receipt.clone();
    rewrite_ref(
        root,
        &mut failed_source,
        "source_audit",
        "validation_artifacts/ultragoal-audit/validator-receipt.json",
        &json!({
            "status":"fail",
            "target_revision":{"kind":"package_digest","value":current},
            "claim_ceiling":"withheld_or_blocked",
            "supported_claim_classes":[],
            "blocked_claim_classes":cases::blocked_claims()
        }),
    );
    support::write_proof(root, &failed_source);
    let source_failures = crate::audit::final_packet::claim_guard_failures(root, store);
    assert!(source_failures.is_empty(), "{source_failures:?}");
    failed_source
}

fn assert_source_audit_pass_self_write_is_not_circular_failure(
    root: &Path,
    store: &crate::schema_catalog::SchemaStore,
    proof: &Value,
    current: &str,
) {
    support::write_json(
        &root.join("validation_artifacts/ultragoal-audit/validator-receipt.json"),
        &json!({
            "status":"pass",
            "generated_at":"2026-06-29T00:00:02Z",
            "target_revision":{"kind":"package_digest","value":current},
            "claim_ceiling":"source_audit_pass_source_local_only",
            "supported_claim_classes":["source_local_audit_checks", "red_fixture_report"],
            "blocked_claim_classes":cases::blocked_claims()
        }),
    );
    support::write_proof(root, proof);
    let failures = crate::audit::final_packet::claim_guard_failures(root, store);
    assert!(failures.is_empty(), "{failures:?}");
}

fn assert_source_audit_self_rewrite_is_not_circular_failure(
    root: &Path,
    store: &crate::schema_catalog::SchemaStore,
    proof: &Value,
    current: &str,
) {
    support::write_json(
        &root.join("validation_artifacts/ultragoal-audit/validator-receipt.json"),
        &json!({
            "status":"fail",
            "generated_at":"2026-06-29T00:00:01Z",
            "target_revision":{"kind":"package_digest","value":current},
            "claim_ceiling":"withheld_or_blocked",
            "supported_claim_classes":[],
            "blocked_claim_classes":cases::blocked_claims()
        }),
    );
    let mut proof = proof.clone();
    proof["source_audit"]["self_rewriting_authority"] =
        json!("source_audit_command_writes_validator_receipt");
    support::write_proof(root, &proof);
    let failures = crate::audit::final_packet::claim_guard_failures(root, store);
    assert!(failures.is_empty(), "{failures:?}");
}

fn assert_source_audit_guard_ref_boundary_failures(
    root: &Path,
    store: &crate::schema_catalog::SchemaStore,
    proof: &Value,
) {
    assert_guard_failure(
        root,
        store,
        mutate(proof, |value| {
            value
                .as_object_mut()
                .expect("proof object")
                .remove("source_audit");
        }),
        "final_packet_proof_ref_missing:source_audit",
    );

    assert_guard_failure(
        root,
        store,
        mutate(proof, |value| {
            value["source_audit"]["path"] = json!("../outside.json");
        }),
        "final_packet_proof_ref_path_invalid:source_audit",
    );

    let source_path = "validation_artifacts/ultragoal-audit/validator-receipt.json";
    std::fs::write(root.join(source_path), "{").expect("malformed source audit rewrite");
    assert_guard_failure(
        root,
        store,
        proof.clone(),
        "final_packet_proof_ref_digest_mismatch:source_audit",
    );
}

fn assert_weak_source_audit_failures(
    root: &Path,
    store: &crate::schema_catalog::SchemaStore,
    receipt: &Value,
    current: &str,
) {
    let mut weak_failed_source = receipt.clone();
    rewrite_ref(
        root,
        &mut weak_failed_source,
        "source_audit",
        "validation_artifacts/ultragoal-audit/validator-receipt.json",
        &json!({"status":"fail","target_revision":{"kind":"package_digest","value":current}}),
    );
    assert_guard_failure(
        root,
        store,
        weak_failed_source,
        "final_packet_proof_source_audit_fail_claim_ceiling_not_blocking",
    );
}

fn assert_source_audit_status_failures(
    root: &Path,
    store: &crate::schema_catalog::SchemaStore,
    receipt: &Value,
    current: &str,
) {
    for (source, expected) in cases::failure_cases(current) {
        let mut proof = receipt.clone();
        rewrite_ref(
            root,
            &mut proof,
            "source_audit",
            "validation_artifacts/ultragoal-audit/validator-receipt.json",
            &source,
        );
        assert_guard_failure(root, store, proof, expected);
    }
}

pub(super) fn mutate(value: &Value, edit: impl FnOnce(&mut Value)) -> Value {
    let mut value = value.clone();
    edit(&mut value);
    value
}

pub(super) fn rewrite_ref(root: &Path, proof: &mut Value, key: &str, path: &str, value: &Value) {
    support::write_json(&root.join(path), value);
    proof[key]["path"] = json!(path);
    proof[key]["digest"] = json!(crate::digest::file(&root.join(path)).expect("ref digest"));
    proof[key]["status"] = value["status"].clone();
}

pub(super) fn assert_guard_failure(
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
