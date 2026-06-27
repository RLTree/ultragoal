use crate::audit::contract::{ClassifierKind, Failure, SemanticClassificationReceipt};
use crate::claim_semantics::str_field;
use crate::digest;
use serde_json::Value;

pub(super) fn check(
    claim: &Value,
    receipt: &SemanticClassificationReceipt,
    out: &mut Vec<Failure>,
) {
    match receipt.classifier_implementation_kind {
        ClassifierKind::Model => {
            if receipt.provider_model.is_none()
                || receipt
                    .prompt_contract_digest
                    .as_deref()
                    .is_none_or(bad_digest)
                || !evidence_is(receipt, "external_model_output")
            {
                push(out, claim, "model_external_attestation_required");
            }
        }
        ClassifierKind::HumanReviewer => {
            if receipt.provider_model.is_some()
                || receipt.prompt_contract_digest.is_some()
                || !evidence_is(receipt, "human_review_attestation")
            {
                push(out, claim, "human_external_attestation_required");
            }
        }
        ClassifierKind::DeterministicBackstop => {
            if receipt.provider_model.is_some() || receipt.classifier_evidence.is_some() {
                push(out, claim, "deterministic");
            }
        }
    }
}

fn evidence_is(receipt: &SemanticClassificationReceipt, kind: &str) -> bool {
    receipt
        .classifier_evidence
        .as_ref()
        .is_some_and(|evidence| {
            evidence.evidence_type == kind
                && !bad_digest(&evidence.digest)
                && evidence.summary.trim().len() >= 12
        })
}

fn bad_digest(value: &str) -> bool {
    value == digest::ZERO || !value.starts_with("sha256:") || value.len() != 71
}

fn push(out: &mut Vec<Failure>, claim: &Value, detail: &str) {
    out.push(Failure::new(
        "claim-status-ceiling",
        "semantic_classification_receipt_malformed",
        format!("{}:{detail}", str_field(claim, "id")),
    ));
}
