use crate::audit::contract::Failure;
use serde_json::json;

fn errors(out: &[Failure]) -> Vec<&str> {
    out.iter().map(|failure| failure.error.as_str()).collect()
}

#[test]
fn product_cohesion_receipt_policy_rejects_substitutes_and_missing_bindings() {
    let claim = json!({
        "id":"PC10",
        "title":"Product cohesion same-surface proof",
        "claim_ceiling_effect":"included",
        "requires_product_cohesion":true,
        "evidence":[
            {
                "id":"ui-live",
                "kind":"live_beneficial_e2e",
                "surface":"ui_browser",
                "path":"validation_artifacts/product/ui-live.json",
                "digest":crate::self_tests::boundaries::workspace_fixtures::sha('a')
            },
            {
                "id":"harness-path",
                "kind":"harness_receipt",
                "surface":"harness",
                "path":"validation_artifacts/product/harness-path.json",
                "digest":crate::self_tests::boundaries::workspace_fixtures::sha('b')
            },
            {
                "id":"exception-path",
                "kind":"human_review_queue_exception",
                "surface":"product::cohesion",
                "path":"validation_artifacts/product/exception.json",
                "digest":crate::self_tests::boundaries::workspace_fixtures::sha('c')
            },
            {
                "id":"mock-product",
                "kind":"product_cohesion_receipt",
                "surface":"product::cohesion",
                "path":"validation_artifacts/product/mock-proof.json",
                "digest":crate::digest::ZERO,
                "product_cohesion_task":{
                    "primary_journey_id":"fixture journey",
                    "ui_evidence_paths":["validation_artifacts/product/ui-live.json"],
                    "exhausted_harness_evidence_paths":["validation_artifacts/product/harness-path.json"],
                    "expected_interruption_rate":"low"
                }
            },
            {
                "id":"wrong-ui",
                "kind":"product_cohesion_receipt",
                "surface":"product::cohesion",
                "path":"validation_artifacts/product/product-receipt.json",
                "digest":crate::self_tests::boundaries::workspace_fixtures::sha('d'),
                "product_cohesion_task":{
                    "primary_journey_id":"current journey",
                    "ui_evidence_paths":["validation_artifacts/product/missing-ui.json"],
                    "exhausted_harness_evidence_paths":["validation_artifacts/product/harness-path.json"],
                    "expected_interruption_rate":"low"
                }
            },
            {
                "id":"missing-exception",
                "kind":"product_cohesion_receipt",
                "surface":"product::cohesion",
                "path":"validation_artifacts/product/product-receipt.json",
                "digest":crate::self_tests::boundaries::workspace_fixtures::sha('e'),
                "product_cohesion_task":{
                    "primary_journey_id":"current journey",
                    "ui_evidence_paths":["validation_artifacts/product/ui-live.json"],
                    "exhausted_harness_evidence_paths":["validation_artifacts/product/harness-path.json"],
                    "expected_interruption_rate":"frequent",
                    "human_review_queue_exception":{
                        "evidence_path":"validation_artifacts/product/missing-exception.json"
                    }
                }
            },
            {
                "id":"overuse",
                "kind":"product_cohesion_receipt",
                "surface":"product::cohesion",
                "path":"validation_artifacts/product/product-receipt.json",
                "digest":crate::self_tests::boundaries::workspace_fixtures::sha('f'),
                "product_cohesion_task":{
                    "primary_journey_id":"current journey",
                    "ui_evidence_paths":["validation_artifacts/product/ui-live.json"],
                    "exhausted_harness_evidence_paths":["validation_artifacts/product/harness-path.json"],
                    "expected_interruption_rate":"unknown"
                }
            },
            {
                "id":"missing-harness",
                "kind":"product_cohesion_receipt",
                "surface":"product::cohesion",
                "path":"validation_artifacts/product/product-receipt.json",
                "digest":crate::self_tests::boundaries::workspace_fixtures::sha('0'),
                "product_cohesion_task":{
                    "primary_journey_id":"current journey",
                    "ui_evidence_paths":["validation_artifacts/product/ui-live.json"],
                    "exhausted_harness_evidence_paths":["validation_artifacts/product/missing-harness.json"],
                    "expected_interruption_rate":"low"
                }
            },
            {
                "id":"valid-product",
                "kind":"product_cohesion_receipt",
                "surface":"product::cohesion",
                "path":"validation_artifacts/product/product-receipt.json",
                "digest":crate::self_tests::boundaries::workspace_fixtures::sha('1'),
                "product_cohesion_task":{
                    "primary_journey_id":"current journey",
                    "ui_evidence_paths":["validation_artifacts/product/ui-live.json"],
                    "exhausted_harness_evidence_paths":["validation_artifacts/product/harness-path.json"],
                    "expected_interruption_rate":"frequent",
                    "human_review_queue_exception":{
                        "evidence_path":"validation_artifacts/product/exception.json"
                    }
                }
            }
        ]
    });
    let failures = crate::claim_semantics::product::cohesion::product_cohesion_failures(&claim);
    let got = errors(&failures);
    for expected in [
        "product_cohesion_placeholder_or_mock_proof",
        "product_ui_journey_evidence_mismatch",
        "product_human_attention_exception_evidence_missing",
        "product_human_attention_overuse",
        "product_harness_paths_not_exhausted",
    ] {
        assert!(got.contains(&expected), "{expected}: {failures:?}");
    }
    assert!(
        !failures
            .iter()
            .any(|failure| failure.detail == "valid-product"),
        "{failures:?}"
    );
}
