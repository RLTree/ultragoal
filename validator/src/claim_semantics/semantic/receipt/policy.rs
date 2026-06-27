use crate::audit::contract::{Failure, SemanticClass, SemanticClassificationReceipt};
use crate::claim_semantics::claim::proof;
use crate::claim_semantics::product::cohesion;
use crate::claim_semantics::str_field;
use crate::digest;
use serde_json::Value;
use std::path::Path;

const RECEIPT_SCHEMA: &str = "harness-ultragoal.semantic-classification-receipt.v1";
const CONTRACT_ID: &str = "ultragoal-semantic-classification";
const CONTRACT_VERSION: &str = "v1";

pub fn check_semantic_receipts(claim: &Value, ready: &Value, root: &Path, out: &mut Vec<Failure>) {
    if str_field(claim, "claim_ceiling_effect") != "included"
        && !crate::claim_semantics::good_status(&str_field(claim, "status"))
    {
        return;
    }
    let receipts = claim
        .get("semantic_classification_receipts")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if receipts.is_empty() {
        out.push(Failure::new(
            "claim-status-ceiling",
            "semantic_classification_receipt_missing",
            str_field(claim, "id"),
        ));
        return;
    }
    let expected_text_digest = digest::bytes(proof::claim_text(claim).as_bytes());
    for receipt in receipts {
        match crate::claim_semantics::semantic::receipt::loader::load(root, &receipt) {
            Ok(loaded) => {
                check_one(claim, ready, &loaded.value, &expected_text_digest, out);
                if loaded.inline {
                    out.push(Failure::new(
                        "claim-status-ceiling",
                        "semantic_classification_receipt_inline_provenance",
                        str_field(claim, "id"),
                    ));
                }
            }
            Err(err) => out.push(Failure::new(
                "claim-status-ceiling",
                "semantic_classification_receipt_malformed",
                err,
            )),
        }
    }
}

fn check_one(
    claim: &Value,
    ready: &Value,
    receipt: &Value,
    expected_digest: &str,
    out: &mut Vec<Failure>,
) {
    let Ok(typed) = serde_json::from_value::<SemanticClassificationReceipt>(receipt.clone()) else {
        out.push(Failure::new(
            "claim-status-ceiling",
            "semantic_classification_receipt_malformed",
            str_field(claim, "id"),
        ));
        return;
    };
    basic_receipt_checks(claim, receipt, expected_digest, &typed, out);
    crate::claim_semantics::semantic::receipt::provenance::check(claim, &typed, out);
    ambiguity_confidence_check(claim, &typed, out);
    product_text_receipt_check(claim, &typed, out);
    structured_applicability_check(claim, &typed, out);
    crate::claim_semantics::semantic::receipt::text::deterministic_backstop_lower_bound(
        claim, &typed, out,
    );
    crate::claim_semantics::semantic::receipt::gates::required_gate_check(
        claim, ready, &typed, out,
    );
    text_receipt_backstop(claim, &typed, out);
}

fn basic_receipt_checks(
    claim: &Value,
    receipt: &Value,
    expected_digest: &str,
    typed: &SemanticClassificationReceipt,
    out: &mut Vec<Failure>,
) {
    if typed.schema != RECEIPT_SCHEMA {
        push(
            out,
            "semantic_classification_receipt_malformed",
            claim,
            "schema",
        );
    }
    if typed.classifier_contract_id != CONTRACT_ID
        || typed.classifier_contract_version != CONTRACT_VERSION
    {
        push(
            out,
            "semantic_classification_receipt_contract_mismatch",
            claim,
            "contract",
        );
    }
    if typed.claim_id != str_field(claim, "id") {
        push(
            out,
            "semantic_classification_receipt_wrong_claim",
            claim,
            "claim_id",
        );
    }
    if typed.canonical_text_digest != expected_digest {
        push(
            out,
            "semantic_classification_receipt_stale",
            claim,
            "text_digest",
        );
    }
    if !typed.actor_disjoint || typed.producer_actor_id == typed.classifier_actor_id {
        push(
            out,
            "semantic_classification_not_actor_disjoint",
            claim,
            "actor",
        );
    }
    if typed.receipt_digest == digest::ZERO || typed.receipt_digest != receipt_digest(receipt) {
        push(
            out,
            "semantic_classification_receipt_malformed",
            claim,
            "receipt_digest",
        );
    }
}

fn ambiguity_confidence_check(
    claim: &Value,
    typed: &SemanticClassificationReceipt,
    out: &mut Vec<Failure>,
) {
    if typed.ambiguity
        && !typed
            .detected_semantic_classes
            .contains(&SemanticClass::AmbiguousNeedsReviewerClassification)
    {
        push(
            out,
            "semantic_classification_ambiguous_without_reviewer",
            claim,
            "ambiguity",
        );
    }
    if typed.confidence < 0.70
        && !typed
            .detected_semantic_classes
            .contains(&SemanticClass::AmbiguousNeedsReviewerClassification)
    {
        push(
            out,
            "semantic_classification_low_confidence",
            claim,
            "confidence",
        );
    }
}

fn receipt_digest(receipt: &Value) -> String {
    let mut normalized = receipt.clone();
    if let Some(obj) = normalized.as_object_mut() {
        obj.insert(
            "receipt_digest".to_string(),
            Value::String(digest::ZERO.to_string()),
        );
    }
    digest::canonical_json(&normalized)
}

fn product_text_receipt_check(
    claim: &Value,
    receipt: &SemanticClassificationReceipt,
    out: &mut Vec<Failure>,
) {
    if cohesion::claim_text_product_applicable(claim)
        && !receipt
            .detected_semantic_classes
            .iter()
            .any(crate::claim_semantics::semantic::receipt::text::is_product_class)
    {
        push(
            out,
            "semantic_classification_contradicts_claim_text",
            claim,
            "product",
        );
    }
}

fn structured_applicability_check(
    claim: &Value,
    receipt: &SemanticClassificationReceipt,
    out: &mut Vec<Failure>,
) {
    let product_app = &claim["product_applicability"];
    if product_app.is_null() {
        return;
    }
    let says_product = ["user_facing", "product_surface", "ui_or_control_surface"]
        .iter()
        .any(|key| product_app.get(*key).and_then(Value::as_bool) == Some(true));
    let has_runtime_only = receipt
        .detected_semantic_classes
        .contains(&SemanticClass::RuntimeCliBackendOnlyEngineOnly);
    if says_product
        && has_runtime_only
        && !receipt
            .detected_semantic_classes
            .iter()
            .any(crate::claim_semantics::semantic::receipt::text::is_product_class)
    {
        push(
            out,
            "semantic_classification_contradicts_structured_claim",
            claim,
            "product_applicability",
        );
    }
}

fn text_receipt_backstop(
    claim: &Value,
    receipt: &SemanticClassificationReceipt,
    out: &mut Vec<Failure>,
) {
    crate::claim_semantics::semantic::receipt::text::text_receipt_backstop(claim, receipt, out);
}

fn push(out: &mut Vec<Failure>, error: &str, claim: &Value, detail: &str) {
    out.push(Failure::new(
        "claim-status-ceiling",
        error,
        format!("{}:{detail}", str_field(claim, "id")),
    ));
}
