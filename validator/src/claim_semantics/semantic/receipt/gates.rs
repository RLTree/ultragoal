use crate::audit::contract::{ClassifierKind, Failure, ProofGate, SemanticClassificationReceipt};
use crate::claim_semantics::str_field;
use serde_json::Value;

pub(crate) fn required_gate_check(
    claim: &Value,
    ready: &Value,
    receipt: &SemanticClassificationReceipt,
    out: &mut Vec<Failure>,
) {
    for gate in
        crate::semantic::receipt::classifier::proof_gates(&receipt.detected_semantic_classes)
    {
        if !receipt.required_proof_gates.contains(&gate) {
            push(
                out,
                "semantic_classification_required_gate_unproven",
                claim,
                &format!("missing_{gate:?}"),
            );
            return;
        }
    }
    let evs = crate::claim_semantics::evidence(claim);
    for gate in &receipt.required_proof_gates {
        let ok = gate_satisfied(gate, &evs, claim, ready, receipt);
        if !ok {
            let error = if matches!(gate, ProofGate::ReviewerClassification) {
                "semantic_classification_ambiguous_without_reviewer"
            } else {
                "semantic_classification_required_gate_unproven"
            };
            push(out, error, claim, &format!("{gate:?}"));
        }
    }
}

fn gate_satisfied(
    gate: &ProofGate,
    evs: &[&Value],
    claim: &Value,
    ready: &Value,
    receipt: &SemanticClassificationReceipt,
) -> bool {
    match gate {
        ProofGate::ProductCohesionReceipt => {
            proof_row(evs, &["product::cohesion"], &["product_cohesion_receipt"])
        }
        ProofGate::UiJourneyEvidence => {
            proof_row(evs, &["ui_browser", "ui_computer"], &["ui_interaction"])
        }
        ProofGate::AccessibilityEvidence => {
            proof_row(evs, &["product::cohesion"], &["product_cohesion_receipt"])
        }
        ProofGate::RuntimeExecution => has_kind(evs, &["runtime_execution", "live_beneficial_e2e"]),
        ProofGate::LiveBeneficialE2e => has_kind(evs, &["live_beneficial_e2e"]),
        ProofGate::InstallVisibilityReceipt => {
            proof_row(evs, &["external"], &["promotion_receipt"])
        }
        ProofGate::PublicationExternalAttestation => {
            proof_row(evs, &["external"], &["promotion_receipt"])
        }
        ProofGate::ReadyForMergeReceipt => ready_joined_to_claim(ready, claim),
        ProofGate::ObservabilityReceipt => {
            proof_row(evs, &["observability"], &["observability_receipt"])
        }
        ProofGate::ReviewerClassification => {
            receipt.classifier_implementation_kind == ClassifierKind::HumanReviewer
                && receipt.classifier_evidence.is_some()
        }
        ProofGate::WithheldOrBacklogClaim => {
            str_field(claim, "claim_ceiling_effect") != "included"
                && !crate::claim_semantics::good_status(&str_field(claim, "status"))
        }
    }
}

fn has_kind(evs: &[&Value], required: &[&str]) -> bool {
    evs.iter()
        .any(|ev| required.contains(&str_field(ev, "kind").as_str()))
}

fn proof_row(evs: &[&Value], surfaces: &[&str], kinds: &[&str]) -> bool {
    crate::claim_semantics::claim::proof::text::proof_row_exists(evs, surfaces, kinds)
}

fn ready_joined_to_claim(ready: &Value, claim: &Value) -> bool {
    ready.get("ready").and_then(Value::as_bool) == Some(true)
        && crate::json_boundary::string_array(ready, "claim_ids").contains(&str_field(claim, "id"))
}

fn push(out: &mut Vec<Failure>, error: &str, claim: &Value, detail: &str) {
    out.push(Failure::new(
        "claim-status-ceiling",
        error,
        format!("{}:{detail}", str_field(claim, "id")),
    ));
}
