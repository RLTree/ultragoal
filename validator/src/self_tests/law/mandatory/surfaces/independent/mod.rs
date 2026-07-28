use super::surface_fixtures::{
    expect_failure, production_law, validator_theater_law, with_independent_verification,
    write_inspected_source, write_json, write_manual_verification, write_specific_red_fixture,
};
use serde_json::{Value, json};

mod receipt_paths;

#[test]
fn independent_verification_rejects_missing_and_unsafe_receipt_paths() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("mandatory-independent-paths");
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":[]}),
    );
    write_manual_verification(&root, "schema-valid");

    let mut missing_path = with_independent_verification(production_law("schema-valid"));
    missing_path["independent_verification"]
        .as_object_mut()
        .expect("independent verification")
        .remove("receipt_path");
    expect_failure(
        &root,
        &missing_path,
        "mandatory_law_independent_verification_receipt_missing:schema-valid",
    );

    for path in [
        "/tmp/manual.json",
        "../validation_artifacts/manual/receipt.json",
        "validation_artifacts/other/receipt.json",
        "validation_artifacts/manual/receipt.txt",
    ] {
        let mut unsafe_path = with_independent_verification(production_law("schema-valid"));
        unsafe_path["independent_verification"]["receipt_path"] = json!(path);
        expect_failure(
            &root,
            &unsafe_path,
            "mandatory_law_independent_verification_receipt_path_invalid:schema-valid",
        );
    }

    std::fs::remove_dir_all(root).expect("cleanup mandatory independent paths");
}

#[test]
fn independent_verification_rejects_shape_substitutes() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("mandatory-independent-shape");
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":[]}),
    );
    write_manual_verification(&root, "schema-valid");

    let mut missing_manual = with_independent_verification(production_law("schema-valid"));
    missing_manual["independent_verification"]
        .as_object_mut()
        .expect("independent verification")
        .remove("source_runtime_manual_required");
    expect_failure(
        &root,
        &missing_manual,
        "mandatory_law_independent_verification_manual_required_missing:schema-valid",
    );

    let mut invalid_scope = with_independent_verification(production_law("schema-valid"));
    invalid_scope["independent_verification"]["verification_scope"] = json!("checklist_only");
    expect_failure(
        &root,
        &invalid_scope,
        "mandatory_law_independent_verification_scope_invalid:schema-valid",
    );

    let mut invalid_authority = with_independent_verification(production_law("schema-valid"));
    invalid_authority["independent_verification"]["authority"] = json!("cli_pass");
    invalid_authority["independent_verification"]["cli_pass_alone_allowed"] = json!(true);
    expect_failure(
        &root,
        &invalid_authority,
        "mandatory_law_independent_verification_authority_invalid:schema-valid",
    );
    expect_failure(
        &root,
        &invalid_authority,
        "mandatory_law_independent_verification_cli_only:schema-valid",
    );

    std::fs::remove_dir_all(root).expect("cleanup mandatory independent shape");
}

#[test]
fn independent_verification_rejects_safe_but_missing_receipt_file() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "mandatory-independent-missing",
    );
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":[]}),
    );
    let law = with_independent_verification(production_law("schema-valid"));
    expect_failure(
        &root,
        &law,
        "mandatory_law_independent_verification_receipt_missing:schema-valid",
    );
    std::fs::remove_dir_all(root).expect("cleanup mandatory independent missing");
}

#[test]
fn independent_verification_rejects_pass_shaped_manual_receipts() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "mandatory-independent-receipt",
    );
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":[]}),
    );
    write_specific_red_fixture(&root, "schema-valid-red", "schema-valid", "schema_dispatch");

    write_manual_receipt(
        &root,
        json!({
            "schema": "wrong",
            "status": "stale",
            "candidate_digest": crate::self_tests::boundaries::workspace_fixtures::sha('9'),
            "law_ids": ["schema-valid"],
            "cli_pass_alone_rejected": false,
            "source_paths": [],
            "runtime_evidence_paths": [],
            "manual_checks": []
        }),
    );
    let law = with_independent_verification(production_law("schema-valid"));
    let failures = crate::audit::mandatory::law::surfaces::receipt_value_failures(&root, &law);
    for expected in [
        "mandatory_law_independent_verification_receipt_wrong_schema:schema-valid",
        "mandatory_law_independent_verification_receipt_not_current:schema-valid",
        "mandatory_law_independent_verification_receipt_digest_mismatch:schema-valid",
        "mandatory_law_independent_verification_receipt_cli_only:schema-valid",
        "mandatory_law_independent_verification_receipt_missing_field:schema-valid:source_paths",
        "mandatory_law_independent_verification_receipt_missing_field:schema-valid:runtime_evidence_paths",
        "mandatory_law_independent_verification_receipt_missing_field:schema-valid:manual_checks",
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
            "law_ids": ["all_mandatory_laws"],
            "cli_pass_alone_rejected": true,
            "source_paths": ["validator/src/audit/mandatory/law/surfaces/mod.rs"],
            "runtime_evidence_paths": [
                "validation_artifacts/manual/parent-source-runtime-verification.json"
            ],
            "manual_checks": [{
                "claim_path": "mandatory-law-surfaces",
                "verification_method": "source_inspection",
                "finding": "all mandatory law coverage is intentionally accepted only by typed manual receipt"
            }]
        }),
    );
    expect_failure(
        &root,
        &law,
        "mandatory_law_independent_verification_receipt_law_missing:schema-valid",
    );

    write_inspected_source(&root);
    write_manual_receipt(
        &root,
        json!({
            "schema": "harness-ultragoal.parent-source-runtime-verification.v1",
            "status": "verified_current",
            "candidate_digest": crate::package::inventory::package_digest(&root)
                .expect("candidate"),
            "law_ids": ["schema-valid"],
            "cli_pass_alone_rejected": true,
            "source_paths": ["validator/src/audit/mandatory/law/surfaces/mod.rs"],
            "runtime_evidence_paths": [
                "validation_artifacts/manual/parent-source-runtime-verification.json"
            ],
            "manual_checks": [{
                "claim_path": "mandatory-law-surfaces/schema-valid",
                "verification_method": "source_inspection",
                "finding": "exact law coverage is required; umbrella all_mandatory_laws is not accepted"
            }]
        }),
    );
    expect_failure(
        &root,
        &law,
        "mandatory_law_specific_guard_missing_red_fixture:schema-valid:schema_dispatch",
    );

    std::fs::remove_dir_all(root).expect("cleanup mandatory independent receipt");
}

#[test]
fn independent_verification_is_targeted_not_universal() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "mandatory-independent-targeted",
    );
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":[]}),
    );
    write_specific_red_fixture(&root, "schema-valid-red", "schema-valid", "schema_dispatch");
    let generic_law = production_law("schema-valid");
    expect_failure(
        &root,
        &generic_law,
        "mandatory_law_specific_guard_missing_red_fixture:schema-valid:schema_dispatch",
    );

    let theater_without_manual = validator_theater_law();
    expect_failure(
        &root,
        &theater_without_manual,
        "mandatory_law_independent_verification_missing:validator-theater-miswire-resistance",
    );

    write_specific_red_fixture(
        &root,
        "validator-theater-miswire-resistance-red",
        "validator-theater-miswire-resistance",
        "cli_pass_without_parent_source_runtime_verification",
    );
    write_manual_verification(&root, "validator-theater-miswire-resistance");
    let theater_with_manual = with_independent_verification(validator_theater_law());
    expect_failure(
        &root,
        &theater_with_manual,
        "mandatory_law_specific_guard_missing_red_fixture:validator-theater-miswire-resistance:cli_pass_without_parent_source_runtime_verification",
    );

    std::fs::remove_dir_all(root).expect("cleanup mandatory independent targeted");
}

fn write_manual_receipt(root: &std::path::Path, value: Value) {
    write_json(
        &root.join("validation_artifacts/manual/parent-source-runtime-verification.json"),
        &value,
    );
}
