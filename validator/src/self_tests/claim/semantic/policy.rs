use crate::audit::contract::Failure;
use serde_json::{Value, json};

fn semantic_receipt(claim_id: &str, title: &str, description: &str) -> Value {
    let text = crate::claim::text::normalized_text(&[title, description]);
    let mut receipt = json!({
        "schema": "wrong.schema",
        "claim_id": format!("{claim_id}-other"),
        "canonical_text_digest": crate::self_tests::boundaries::workspace_fixtures::sha('9'),
        "classifier_contract_id": "wrong-contract",
        "classifier_contract_version": "v0",
        "classifier_implementation_kind": "deterministic_backstop",
        "generated_at": "2026-06-25T00:00:00Z",
        "producer_actor_id": "same",
        "classifier_actor_id": "same",
        "actor_disjoint": false,
        "detected_semantic_classes": ["runtime_cli_backend_only_engine_only"],
        "rationale": "intentionally conflicting semantic receipt",
        "confidence": 0.50,
        "ambiguity": true,
        "required_proof_gates": [],
        "claim_ceiling_recommendation": "withhold",
        "receipt_digest": crate::digest::ZERO
    });
    receipt["classifier_evidence"] = json!({
        "evidence_type": "unexpected_external_output",
        "digest": crate::self_tests::boundaries::workspace_fixtures::sha('8'),
        "summary": "deterministic backstops cannot carry external evidence"
    });
    receipt["canonical_text_digest"] = json!(crate::digest::bytes(text.as_bytes()));
    receipt
}

fn semantic_receipt_with_digest(mut receipt: Value) -> Value {
    receipt["receipt_digest"] = json!(crate::digest::ZERO);
    let digest = crate::digest::canonical_json(&receipt);
    receipt["receipt_digest"] = json!(digest);
    receipt
}

fn valid_runtime_receipt(claim_id: &str, title: &str, description: &str) -> Value {
    let text = crate::claim::text::normalized_text(&[title, description]);
    semantic_receipt_with_digest(json!({
        "schema": "harness-ultragoal.semantic-classification-receipt.v1",
        "claim_id": claim_id,
        "canonical_text_digest": crate::digest::bytes(text.as_bytes()),
        "classifier_contract_id": "ultragoal-semantic-classification",
        "classifier_contract_version": "v1",
        "classifier_implementation_kind": "deterministic_backstop",
        "generated_at": "2026-06-25T00:00:00Z",
        "producer_actor_id": "producer",
        "classifier_actor_id": "classifier",
        "actor_disjoint": true,
        "detected_semantic_classes": ["runtime_cli_backend_only_engine_only"],
        "rationale": "runtime-only semantic receipt",
        "confidence": 0.99,
        "ambiguity": false,
        "required_proof_gates": ["runtime_execution", "ready_for_merge_receipt"],
        "claim_ceiling_recommendation": "withhold",
        "receipt_digest": crate::digest::ZERO
    }))
}

#[test]
fn semantic_receipt_policy_rejects_missing_malformed_and_conflicting_receipts() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("semantic-policy");
    std::fs::create_dir_all(&root).expect("semantic root");
    let ready = json!({});
    let title = "Product dashboard release readiness";
    let description = "A user facing product surface is ready.";
    let mut out = Vec::<Failure>::new();
    let base = json!({
        "id": "CLAIM-SEM",
        "title": title,
        "description": description,
        "status": "pass",
        "claim_ceiling_effect": "included",
        "product_applicability": {"user_facing": true}
    });
    crate::claim_semantics::semantic::receipt::policy::check_semantic_receipts(
        &base, &ready, &root, &mut out,
    );
    assert!(
        out.iter()
            .any(|failure| failure.error == "semantic_classification_receipt_missing")
    );

    let mut malformed = base.clone();
    malformed["semantic_classification_receipts"] = json!([{"path":"../escape.json","digest":crate::self_tests::boundaries::workspace_fixtures::sha('1')}]);
    out.clear();
    crate::claim_semantics::semantic::receipt::policy::check_semantic_receipts(
        &malformed, &ready, &root, &mut out,
    );
    assert!(
        out.iter()
            .any(|failure| failure.error == "semantic_classification_receipt_malformed")
    );

    let mut conflicting = base;
    conflicting["semantic_classification_receipts"] =
        json!([semantic_receipt("CLAIM-SEM", title, description)]);
    out.clear();
    crate::claim_semantics::semantic::receipt::policy::check_semantic_receipts(
        &conflicting,
        &ready,
        &root,
        &mut out,
    );
    let errors = out
        .iter()
        .map(|failure| failure.error.as_str())
        .collect::<Vec<_>>();
    assert!(errors.contains(&"semantic_classification_receipt_contract_mismatch"));
    assert!(errors.contains(&"semantic_classification_receipt_wrong_claim"));
    assert!(errors.contains(&"semantic_classification_not_actor_disjoint"));
    assert!(errors.contains(&"semantic_classification_ambiguous_without_reviewer"));
    assert!(errors.contains(&"semantic_classification_low_confidence"));
    assert!(errors.contains(&"semantic_classification_contradicts_claim_text"));
    assert!(errors.contains(&"semantic_classification_contradicts_structured_claim"));
    assert!(errors.contains(&"semantic_classification_receipt_inline_provenance"));
    std::fs::remove_dir_all(root).expect("cleanup semantic policy");
}

#[test]
fn semantic_receipt_policy_accepts_file_backed_current_receipt_and_rejects_tamper() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("semantic-policy-valid");
    std::fs::create_dir_all(root.join("receipts")).expect("receipts");
    let ready = json!({"ready": true, "claim_ids": ["CLAIM-SEM-VALID"]});
    let title = "Runtime verifier";
    let description = "Backend CLI runtime proof.";
    let receipt = valid_runtime_receipt("CLAIM-SEM-VALID", title, description);
    let receipt_path = root.join("receipts/semantic.json");
    std::fs::write(
        &receipt_path,
        serde_json::to_vec(&receipt).expect("semantic receipt"),
    )
    .expect("write semantic receipt");
    let mut claim = json!({
        "id": "CLAIM-SEM-VALID",
        "title": title,
        "description": description,
        "status": "pass",
        "claim_ceiling_effect": "included",
        "evidence": [{"surface": "runtime_execution", "kind": "runtime_execution"}],
        "semantic_classification_receipts": [{
            "path": "receipts/semantic.json",
            "digest": crate::digest::file(&receipt_path).expect("receipt digest")
        }]
    });
    let mut out = Vec::<Failure>::new();
    crate::claim_semantics::semantic::receipt::policy::check_semantic_receipts(
        &claim, &ready, &root, &mut out,
    );
    assert!(out.is_empty(), "{out:?}");

    let mut tampered = receipt;
    tampered["rationale"] = json!("tampered after receipt digest");
    let tampered_path = root.join("receipts/tampered.json");
    std::fs::write(
        &tampered_path,
        serde_json::to_vec(&tampered).expect("tampered receipt"),
    )
    .expect("write tampered receipt");
    claim["semantic_classification_receipts"] = json!([{
        "path": "receipts/tampered.json",
        "digest": crate::digest::file(&tampered_path).expect("tampered file digest")
    }]);
    crate::claim_semantics::semantic::receipt::policy::check_semantic_receipts(
        &claim, &ready, &root, &mut out,
    );
    assert!(
        out.iter()
            .any(|failure| failure.error == "semantic_classification_receipt_malformed"),
        "{out:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup semantic valid");
}

#[test]
fn json_patch_rejects_malformed_boundary_operations() {
    let doc = json!({
        "items": [{"name": "one"}],
        "object": {"nested": {"value": 1}},
        "scalar": true
    });
    let cases = [
        (json!({}), "json_patch must be an array"),
        (json!([{"op":"replace","value":2}]), "json pointer is empty"),
        (
            json!([{"op":"replace","path":"/items/nope/name","value":"bad"}]),
            "invalid array index nope",
        ),
        (
            json!([{"op":"replace","path":"/items/9/name","value":"bad"}]),
            "array index missing 9",
        ),
        (
            json!([{"op":"replace","path":"/object/missing/value","value":2}]),
            "object key missing missing",
        ),
        (
            json!([{"op":"replace","path":"/scalar/value","value":2}]),
            "json pointer parent is not object or array",
        ),
        (
            json!([{"op":"replace","path":"/items/0/name/deep/value","value":2}]),
            "json pointer parent is not object or array",
        ),
        (
            json!([{"op":"bad","path":"/object/nested/value","value":2}]),
            "unsupported patch op bad",
        ),
        (
            json!([{"op":"replace","path":"/items/x","value":2}]),
            "invalid array index x",
        ),
        (
            json!([{"op":"replace","path":"/items/9","value":2}]),
            "array index missing 9",
        ),
        (
            json!([{"op":"bad","path":"/items/0","value":2}]),
            "unsupported patch op bad",
        ),
    ];
    for (ops, expected) in cases {
        let err = crate::claim_semantics::apply_patch(&doc, &ops).expect_err("bad patch rejected");
        assert!(err.contains(expected), "{expected}: {err}");
    }

    let escaped = crate::claim_semantics::apply_patch(
        &json!({"a/b":{"t~v":1}}),
        &json!([{"op":"replace","path":"/a~1b/t~0v","value":2}]),
    )
    .expect("escaped pointer applies");
    assert_eq!(escaped["a/b"]["t~v"], 2);
}
