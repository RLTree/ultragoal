use serde_json::{Value, json};
use std::path::Path;

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent dir");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
}

fn receipt() -> Value {
    json!({
        "claim_ceiling":{
            "unsupported":[{"claim_id":"production_readiness"}]
        },
        "materiality_gate":{"claims":["product release readiness"]}
    })
}

fn required_substitutions() -> Vec<&'static str> {
    vec![
        "generic_product_simplicity_approval",
        "product::cohesion",
        "install_success",
        "package_publication",
        "first_use",
        "smoke_test",
        "fixture_pass",
        "reviewer_agreement",
        "happy_path",
        "receipt_only",
    ]
}

fn dimensions() -> Vec<&'static str> {
    vec![
        "audience",
        "job",
        "context",
        "outcome",
        "accessibility",
        "cognitive_load",
        "recovery_burden",
        "continuance",
        "real_use_evidence",
        "product_success_substitution_rejection",
    ]
}

fn row(path: &str, digest: &str, generated_at: &str) -> Value {
    json!({
        "agent_type":"harness_product_simplicity_falsifier",
        "product_fitness_required":true,
        "product_fitness_owner":"product_simplicity_falsifier",
        "product_fitness_receipt_digest":digest,
        "product_fitness_claim_ids":["production_readiness"],
        "substitution_rejections_reviewed":required_substitutions(),
        "product_fitness_disposition":{
            "owner_persona":"product_simplicity_falsifier",
            "owner_agent_type":"harness_product_simplicity_falsifier",
            "applies_to_product_impacting_claims":true,
            "claim_ids_reviewed":["production_readiness"],
            "generic_product_simplicity_approval_only":false,
            "substitution_rejection":true,
            "dimensions_checked":dimensions(),
            "receipt":{"path":path,"digest":digest},
            "receipt_generated_at":generated_at
        }
    })
}

fn pf_errors(root: &Path, row: &Value) -> Vec<String> {
    let mut out = Vec::new();
    crate::review::round::product::fitness::disposition_errors(
        root,
        row,
        &receipt(),
        "product_simplicity_falsifier",
        &mut out,
    );
    out.into_iter().map(|failure| failure.error).collect()
}

#[test]
fn product_fitness_receipt_staleness_paths_fail_closed() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("review-pf-stale-paths");
    assert!(
        pf_errors(&root, &row("../escape.json", &crate::digest::ZERO, ""))
            .contains(&"review_round_product_fitness_disposition_stale".to_string())
    );

    let receipt_path = root.join("validation_artifacts/harness/product-fitness-receipt.json");
    std::fs::create_dir_all(receipt_path.parent().unwrap()).expect("pf parent");
    std::fs::write(&receipt_path, "{").expect("bad pf receipt");
    let bad_digest = crate::digest::file(&receipt_path).expect("bad digest");
    assert!(
        pf_errors(
            &root,
            &row(
                "validation_artifacts/harness/product-fitness-receipt.json",
                &bad_digest,
                "2026-06-26T00:00:00Z"
            )
        )
        .contains(&"review_round_product_fitness_disposition_stale".to_string())
    );

    write_json(
        &receipt_path,
        &json!({"generated_at":"2026-06-26T00:00:00Z"}),
    );
    let digest = crate::digest::file(&receipt_path).expect("digest");
    let mut stale_row = row(
        "validation_artifacts/harness/product-fitness-receipt.json",
        &digest,
        "2026-06-25T00:00:00Z",
    );
    assert!(
        pf_errors(&root, &stale_row)
            .contains(&"review_round_product_fitness_disposition_stale".to_string())
    );

    stale_row["product_fitness_receipt_digest"] =
        json!(crate::self_tests::boundaries::workspace_fixtures::sha('9'));
    assert!(
        pf_errors(&root, &stale_row)
            .contains(&"review_round_product_fitness_disposition_stale".to_string())
    );

    let mut mismatched_file_row = row(
        "validation_artifacts/harness/product-fitness-receipt.json",
        &digest,
        "2026-06-26T00:00:00Z",
    );
    mismatched_file_row["product_fitness_disposition"]["receipt"]["digest"] = json!(digest);
    std::fs::write(&receipt_path, b"{\"generated_at\":\"changed\"}").expect("change receipt");
    assert!(
        pf_errors(&root, &mismatched_file_row)
            .contains(&"review_round_product_fitness_disposition_stale".to_string())
    );
    std::fs::remove_dir_all(root).expect("cleanup pf stale paths");
}

#[test]
fn product_fitness_review_disposition_rejects_missing_unowned_and_substituted_paths() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("review-pf-disposition");
    std::fs::create_dir_all(&root).expect("pf disposition root");
    let empty = json!({});
    let got = pf_errors(&root, &empty);
    for expected in [
        "review_round_product_fitness_required_missing",
        "review_round_product_fitness_owner_missing",
        "review_round_product_fitness_receipt_binding_missing",
        "review_round_product_fitness_disposition_missing",
    ] {
        assert!(got.contains(&expected.to_string()), "{expected}: {got:?}");
    }

    let mut unowned = row(
        "validation_artifacts/harness/product-fitness-receipt.json",
        &crate::self_tests::boundaries::workspace_fixtures::sha('a'),
        "2026-06-26T00:00:00Z",
    );
    unowned["agent_type"] = json!("wrong_agent");
    unowned["product_fitness_disposition"]["owner_persona"] = json!("wrong_persona");
    unowned["product_fitness_disposition"]["owner_agent_type"] = json!("wrong_agent");
    unowned["product_fitness_disposition"]["applies_to_product_impacting_claims"] = json!(false);
    let got = pf_errors(&root, &unowned);
    assert!(got.contains(&"review_round_product_fitness_disposition_unowned".to_string()));

    let mut unbound = row(
        "validation_artifacts/harness/product-fitness-receipt.json",
        &crate::self_tests::boundaries::workspace_fixtures::sha('b'),
        "2026-06-26T00:00:00Z",
    );
    unbound["product_fitness_claim_ids"] = json!(["production_readiness"]);
    unbound["product_fitness_disposition"]["claim_ids_reviewed"] = json!(["other_claim"]);
    let got = pf_errors(&root, &unbound);
    assert!(got.contains(&"review_round_product_fitness_claim_binding_missing".to_string()));

    let mut substituted = row(
        "validation_artifacts/harness/product-fitness-receipt.json",
        &crate::self_tests::boundaries::workspace_fixtures::sha('c'),
        "2026-06-26T00:00:00Z",
    );
    substituted["substitution_rejections_reviewed"] =
        json!(["generic_product_simplicity_approval"]);
    substituted["product_fitness_disposition"]["generic_product_simplicity_approval_only"] =
        json!(true);
    substituted["product_fitness_disposition"]["substitution_rejection"] = json!(false);
    substituted["product_fitness_disposition"]["dimensions_checked"] = json!(["audience"]);
    let got = pf_errors(&root, &substituted);
    assert!(
        got.contains(&"review_round_product_fitness_generic_approval_substitution".to_string())
    );

    let ignored = row(
        "validation_artifacts/harness/product-fitness-receipt.json",
        &crate::self_tests::boundaries::workspace_fixtures::sha('d'),
        "2026-06-26T00:00:00Z",
    );
    let mut out = Vec::new();
    crate::review::round::product::fitness::disposition_errors(
        &root,
        &ignored,
        &json!({"claim_ceiling":{"unsupported":[]},"materiality_gate":{"claims":["docs only"]}}),
        "contract_claim_falsifier",
        &mut out,
    );
    assert_eq!(out.len(), 0);
    std::fs::remove_dir_all(root).expect("cleanup pf disposition");
}

#[test]
fn review_round_artifact_binding_rejects_shallow_or_duplicate_evidence() {
    let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let mut failures = Vec::new();
    crate::review::round::artifacts::artifact_binding_errors(
        &root,
        &json!({
            "prompt_packet":{"path":"agents/product-simplicity-falsifier.md","digest":crate::digest::file(&root.join("agents/product-simplicity-falsifier.md")).unwrap()},
            "review_report":{"path":"custom-agents/harness-product-simplicity-falsifier.toml","digest":crate::digest::file(&root.join("custom-agents/harness-product-simplicity-falsifier.toml")).unwrap()},
            "evidence_paths_checked":["README.md","README.md"]
        }),
        "product_simplicity_falsifier",
        &mut failures,
    );
    assert!(
        failures
            .iter()
            .any(|failure| { failure.error == "review_round_insufficient_execution_evidence" })
    );
}
