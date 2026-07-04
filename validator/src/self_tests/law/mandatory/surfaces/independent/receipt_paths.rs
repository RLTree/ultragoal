use super::*;

#[test]
fn rejects_stale_or_private_evidence_paths() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "mandatory-independent-evidence-paths",
    );
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":[]}),
    );
    write_specific_red_fixture(&root, "schema-valid-red", "schema-valid", "schema_dispatch");
    let law = with_independent_verification(production_law("schema-valid"));

    write_manual_receipt(
        &root,
        json!({
            "schema": "harness-ultragoal.parent-source-runtime-verification.v1",
            "status": "verified_current",
            "candidate_digest": crate::package::inventory::package_digest(&root)
                .expect("candidate"),
            "law_ids": ["schema-valid"],
            "cli_pass_alone_rejected": true,
            "source_paths": [
                "validator/src/audit/mandatory/law/surfaces/manual_runtime_boundary.rs"
            ],
            "runtime_evidence_paths": [
                "validation_artifacts/manual/missing-runtime-proof.json"
            ],
            "manual_checks": [{
                "claim_path": "mandatory-law-surfaces/schema-valid",
                "verification_method": "source_inspection",
                "finding": "stale paths must not support manual verification"
            }]
        }),
    );
    let failures = crate::audit::mandatory::law::surfaces::receipt_value_failures(&root, &law);
    for expected in [
        "mandatory_law_independent_verification_receipt_path_missing:schema-valid:source_paths",
        "mandatory_law_independent_verification_receipt_path_missing:schema-valid:runtime_evidence_paths",
    ] {
        assert!(
            failures.iter().any(|failure| failure == expected),
            "{expected}: {failures:?}"
        );
    }

    let private_source_path = format!("/{}/tree/private.rs", "Users");
    write_manual_receipt(
        &root,
        json!({
            "schema": "harness-ultragoal.parent-source-runtime-verification.v1",
            "status": "verified_current",
            "candidate_digest": crate::package::inventory::package_digest(&root)
                .expect("candidate"),
            "law_ids": ["schema-valid"],
            "cli_pass_alone_rejected": true,
            "source_paths": [private_source_path],
            "runtime_evidence_paths": ["../manual/receipt.json"],
            "manual_checks": [{
                "claim_path": "mandatory-law-surfaces/schema-valid",
                "verification_method": "source_inspection",
                "finding": "absolute and traversal paths must fail"
            }]
        }),
    );
    let failures = crate::audit::mandatory::law::surfaces::receipt_value_failures(&root, &law);
    for expected in [
        "mandatory_law_independent_verification_receipt_path_invalid:schema-valid:source_paths",
        "mandatory_law_independent_verification_receipt_path_invalid:schema-valid:runtime_evidence_paths",
    ] {
        assert!(
            failures.iter().any(|failure| failure == expected),
            "{expected}: {failures:?}"
        );
    }

    write_manual_receipt(
        &root,
        json!({
            "schema": "harness-ultragoal.parent-source-runtime-verification.v1",
            "status": "verified_current",
            "candidate_digest": crate::package::inventory::package_digest(&root)
                .expect("candidate"),
            "law_ids": ["schema-valid"],
            "cli_pass_alone_rejected": true,
            "source_paths": ["file:validator/src/audit/mandatory/law/surfaces/mod.rs"],
            "runtime_evidence_paths": ["~/.harness/manual.json"],
            "manual_checks": [{
                "claim_path": "mandatory-law-surfaces/schema-valid",
                "verification_method": "source_inspection",
                "finding": "scheme and home-shortcut paths must fail"
            }]
        }),
    );
    let failures = crate::audit::mandatory::law::surfaces::receipt_value_failures(&root, &law);
    for expected in [
        "mandatory_law_independent_verification_receipt_path_invalid:schema-valid:source_paths",
        "mandatory_law_independent_verification_receipt_path_invalid:schema-valid:runtime_evidence_paths",
    ] {
        assert!(
            failures.iter().any(|failure| failure == expected),
            "{expected}: {failures:?}"
        );
    }

    std::fs::remove_dir_all(root).expect("cleanup mandatory independent evidence paths");
}
