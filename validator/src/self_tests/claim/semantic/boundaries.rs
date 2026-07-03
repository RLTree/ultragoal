use serde_json::{Value, json};

fn with_receipt_digest(mut receipt: Value) -> Value {
    receipt["receipt_digest"] = json!(crate::digest::ZERO);
    let digest = crate::digest::canonical_json(&receipt);
    receipt["receipt_digest"] = json!(digest);
    receipt
}

fn semantic_claim(id: &str, title: &str, description: &str, receipt: Value) -> Value {
    json!({
        "id": id,
        "title": title,
        "description": description,
        "status": "pass",
        "claim_ceiling_effect": "included",
        "evidence": [{"kind":"runtime_execution","surface":"runtime"}],
        "semantic_classification_receipts": [receipt]
    })
}

fn receipt(id: &str, title: &str, description: &str, kind: &str, classes: Value) -> Value {
    with_receipt_digest(json!({
        "schema": "harness-ultragoal.semantic-classification-receipt.v1",
        "claim_id": id,
        "canonical_text_digest": crate::digest::bytes(
            crate::claim::text::normalized_text(&[title, description]).as_bytes()
        ),
        "classifier_contract_id": "ultragoal-semantic-classification",
        "classifier_contract_version": "v1",
        "classifier_implementation_kind": kind,
        "generated_at": "2026-06-25T00:00:00Z",
        "producer_actor_id": "producer",
        "classifier_actor_id": "classifier",
        "actor_disjoint": true,
        "detected_semantic_classes": classes,
        "rationale": "semantic boundary fixture",
        "confidence": 0.99,
        "ambiguity": false,
        "required_proof_gates": ["runtime_execution", "ready_for_merge_receipt"],
        "claim_ceiling_recommendation": "withhold",
        "receipt_digest": crate::digest::ZERO
    }))
}

fn semantic_errors(claim: &Value) -> Vec<String> {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("semantic-boundary");
    std::fs::create_dir_all(&root).expect("semantic boundary root");
    let ready = json!({"ready":true,"claim_ids":[claim["id"].as_str().unwrap_or_default()]});
    let mut out = Vec::new();
    crate::claim_semantics::semantic::receipt::policy::check_semantic_receipts(
        claim, &ready, &root, &mut out,
    );
    let _ = std::fs::remove_dir_all(root);
    out.into_iter()
        .map(|failure| format!("{}:{}", failure.error, failure.detail))
        .collect()
}

#[test]
fn semantic_receipt_provenance_rejects_model_human_and_deterministic_substitutes() {
    let id = "CLAIM-SEM-PROVENANCE";
    let title = "Backend runtime verifier";
    let description = "CLI backend runtime proof.";
    let mut model_missing = receipt(
        id,
        title,
        description,
        "model",
        json!(["runtime_cli_backend_only_engine_only"]),
    );
    let errors = semantic_errors(&semantic_claim(
        id,
        title,
        description,
        model_missing.clone(),
    ));
    assert!(
        errors
            .iter()
            .any(|error| error.contains("model_external_attestation_required")),
        "{errors:?}"
    );

    model_missing["provider_model"] =
        json!({"provider":"test","model":"semantic-falsifier","version":"1"});
    model_missing["prompt_contract_digest"] =
        json!(crate::self_tests::boundaries::workspace_fixtures::sha('a'));
    model_missing["classifier_evidence"] = json!({
        "evidence_type":"external_model_output",
        "digest":crate::self_tests::boundaries::workspace_fixtures::sha('b'),
        "summary":"external model output attestation"
    });
    let model_ok = with_receipt_digest(model_missing);
    let errors = semantic_errors(&semantic_claim(id, title, description, model_ok));
    assert!(
        !errors
            .iter()
            .any(|error| error.contains("model_external_attestation_required")),
        "{errors:?}"
    );

    let mut human_bad = receipt(
        id,
        title,
        description,
        "human_reviewer",
        json!(["runtime_cli_backend_only_engine_only"]),
    );
    human_bad["provider_model"] =
        json!({"provider":"not-human","model":"substitute","version":"1"});
    let errors = semantic_errors(&semantic_claim(id, title, description, human_bad));
    assert!(
        errors
            .iter()
            .any(|error| error.contains("human_external_attestation_required")),
        "{errors:?}"
    );

    let mut deterministic_bad = receipt(
        id,
        title,
        description,
        "deterministic_backstop",
        json!(["runtime_cli_backend_only_engine_only"]),
    );
    deterministic_bad["classifier_evidence"] = json!({
        "evidence_type":"external_model_output",
        "digest":crate::self_tests::boundaries::workspace_fixtures::sha('c'),
        "summary":"external evidence cannot belong to deterministic backstop"
    });
    let errors = semantic_errors(&semantic_claim(id, title, description, deterministic_bad));
    assert!(
        errors.iter().any(|error| error.contains(":deterministic")),
        "{errors:?}"
    );
}

#[test]
fn semantic_receipt_text_rejects_install_publication_and_backstop_mismatches() {
    let id = "CLAIM-SEM-TEXT";
    let title = "Plugin installed visible and marketplace published";
    let description = "The plugin is selectable in Codex and published in the catalog.";
    let claim = semantic_claim(
        id,
        title,
        description,
        receipt(
            id,
            title,
            description,
            "deterministic_backstop",
            json!(["runtime_cli_backend_only_engine_only"]),
        ),
    );
    let errors = semantic_errors(&claim);
    assert!(
        errors
            .iter()
            .any(|error| error.contains("install_visibility")),
        "{errors:?}"
    );
    assert!(
        errors.iter().any(|error| error.contains("publication")),
        "{errors:?}"
    );
    assert!(
        errors
            .iter()
            .any(|error| error.contains("deterministic_backstop")),
        "{errors:?}"
    );
}

#[test]
fn semantic_receipt_policy_rejects_typed_malformed_and_stale_text_digest() {
    let id = "CLAIM-SEM-BAD-TYPE";
    let title = "Runtime verifier";
    let description = "Backend CLI runtime proof.";
    let mut invalid_class = receipt(
        id,
        title,
        description,
        "deterministic_backstop",
        json!(["not_a_semantic_class"]),
    );
    invalid_class["receipt_digest"] = json!(crate::digest::ZERO);
    let errors = semantic_errors(&semantic_claim(id, title, description, invalid_class));
    assert!(
        errors
            .iter()
            .any(|error| error.contains("semantic_classification_receipt_malformed")),
        "{errors:?}"
    );

    let stale_id = "CLAIM-SEM-STALE";
    let mut stale = receipt(
        stale_id,
        title,
        description,
        "deterministic_backstop",
        json!(["runtime_cli_backend_only_engine_only"]),
    );
    stale["canonical_text_digest"] =
        json!(crate::self_tests::boundaries::workspace_fixtures::sha('d'));
    let stale = with_receipt_digest(stale);
    let errors = semantic_errors(&semantic_claim(stale_id, title, description, stale));
    assert!(
        errors
            .iter()
            .any(|error| error.contains("semantic_classification_receipt_stale")),
        "{errors:?}"
    );
}
