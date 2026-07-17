use serde_json::json;

fn errors(out: &[crate::audit::contract::Failure]) -> Vec<&str> {
    out.iter().map(|failure| failure.error.as_str()).collect()
}

#[test]
fn semantic_review_and_red_observation_edges_cover_success_paths() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "session_and_review-semantic-review-red",
    );
    std::fs::create_dir_all(&root).expect("root");
    let title = "Backend runtime verifier";
    let description = "CLI backend runtime proof.";
    let text = crate::claim::text::normalized_text(&[title, description]);
    let mut receipt = json!({
        "schema": "harness-ultragoal.semantic-classification-receipt.v1",
        "claim_id": "CLAIM-HUMAN",
        "canonical_text_digest": crate::digest::bytes(text.as_bytes()),
        "classifier_contract_id": "ultragoal-semantic-classification",
        "classifier_contract_version": "v1",
        "classifier_implementation_kind": "human_reviewer",
        "generated_at": "2026-06-25T00:00:00Z",
        "producer_actor_id": "producer",
        "classifier_actor_id": "reviewer",
        "actor_disjoint": true,
        "detected_semantic_classes": ["runtime_cli_backend_only_engine_only"],
        "rationale": "human review attestation for runtime-only semantics",
        "confidence": 0.99,
        "ambiguity": false,
        "classifier_evidence": {
            "evidence_type":"human_review_attestation",
            "digest":crate::self_tests::boundaries::workspace_fixtures::sha('h'),
            "summary":"human reviewer attestation"
        },
        "required_proof_gates": [],
        "claim_ceiling_recommendation": "withhold",
        "receipt_digest": crate::digest::ZERO
    });
    receipt["receipt_digest"] = json!(crate::digest::canonical_json(&receipt));
    let claim = json!({
        "id":"CLAIM-HUMAN",
        "title":title,
        "description":description,
        "status":"pass",
        "claim_ceiling_effect":"included",
        "semantic_classification_receipts":[receipt]
    });
    let mut semantic = Vec::new();
    crate::claim_semantics::semantic::receipt::policy::check_semantic_receipts(
        &claim,
        &json!({"claim_ids":["CLAIM-HUMAN"]}),
        &root,
        &mut semantic,
    );
    assert!(
        !errors(&semantic).contains(&"semantic_classification_receipt_malformed"),
        "{semantic:?}"
    );

    let repo = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let mut ceiling = crate::self_tests::review::claim_ceiling::receipt();
    ceiling["claim_ceiling"]["unsupported"] = json!([
        {"claim_id":"plugins_ui_visibility"},
        {"claim_id":"install_button_success"},
        {"claim_id":"workspace_public_marketplace_publication"},
        {"claim_id":"real_multilane_dogfood"},
        {"claim_id":"production_readiness"}
    ]);
    let mut review = Vec::new();
    crate::review::round::claim::ceiling::row_authority_errors(
        &repo,
        &ceiling,
        &crate::self_tests::review::claim_ceiling::anchors(),
        &json!({"claim_ceiling_assessment":{
            "authority":"falsification_only_cannot_raise",
            "not_disproven":[
                {"claim_id":"package_static_fixture_proof"},
                {"claim_id":"detached_review_target_archive_identity"}
            ],
            "challenged":[
                {"claim_id":"plugins_ui_visibility"},
                {"claim_id":"install_button_success"},
                {"claim_id":"workspace_public_marketplace_publication"},
                {"claim_id":"real_multilane_dogfood"},
                {"claim_id":"production_readiness"},
                {"claim_id":"external_product_ux_improvement"},
                {"claim_id":"reviewer_runtime_configuration"}
            ]
        }}),
        "claim-falsifier",
        &mut review,
    );
    assert!(
        review
            .iter()
            .any(|failure| failure.error == "review_round_claim_ceiling_missing")
    );
    std::fs::remove_dir_all(root).expect("cleanup session_and_review semantic review red");
}
