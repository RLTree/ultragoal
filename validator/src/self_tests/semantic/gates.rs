use crate::audit::contract::{
    ClassifierEvidence, ClassifierKind, Failure, ProofGate, SemanticClass,
    SemanticClassificationReceipt,
};
use serde_json::json;

fn receipt(
    gates: Vec<ProofGate>,
    detected: Vec<SemanticClass>,
    kind: ClassifierKind,
    evidence: Option<ClassifierEvidence>,
) -> SemanticClassificationReceipt {
    SemanticClassificationReceipt {
        schema: "harness-ultragoal.semantic-classification-receipt.v1".to_string(),
        claim_id: "CLAIM-GATE".to_string(),
        canonical_text_digest: crate::self_tests::boundaries::support::sha('a'),
        classifier_contract_id: "semantic-classifier".to_string(),
        classifier_contract_version: "1".to_string(),
        classifier_implementation_kind: kind,
        provider_model: None,
        prompt_contract_digest: None,
        classifier_evidence: evidence,
        generated_at: "2026-06-26T09:00:00Z".to_string(),
        producer_actor_id: "producer".to_string(),
        classifier_actor_id: "classifier".to_string(),
        actor_disjoint: true,
        detected_semantic_classes: detected,
        rationale: "typed coverage fixture".to_string(),
        confidence: 0.91,
        ambiguity: false,
        required_proof_gates: gates,
        claim_ceiling_recommendation: "withheld_or_blocked".to_string(),
        receipt_digest: crate::self_tests::boundaries::support::sha('b'),
    }
}

fn has_fail(failures: &[Failure], error: &str) -> bool {
    failures.iter().any(|failure| failure.error == error)
}

#[test]
fn semantic_gate_check_accepts_every_supported_gate_shape() {
    let claim = json!({
        "id":"CLAIM-GATE",
        "status":"withheld",
        "claim_ceiling_effect":"withheld_or_blocked",
        "evidence":[
            {"surface":"product::cohesion","kind":"product_cohesion_receipt"},
            {"surface":"ui_browser","kind":"ui_interaction"},
            {"surface":"source","kind":"runtime_execution"},
            {"surface":"source","kind":"live_beneficial_e2e"},
            {"surface":"external","kind":"promotion_receipt"},
            {"surface":"observability","kind":"observability_receipt"}
        ]
    });
    let ready = json!({"ready":true,"claim_ids":["CLAIM-GATE"]});
    let gates = vec![
        ProofGate::ProductCohesionReceipt,
        ProofGate::UiJourneyEvidence,
        ProofGate::AccessibilityEvidence,
        ProofGate::RuntimeExecution,
        ProofGate::LiveBeneficialE2e,
        ProofGate::InstallVisibilityReceipt,
        ProofGate::PublicationExternalAttestation,
        ProofGate::ReadyForMergeReceipt,
        ProofGate::ObservabilityReceipt,
        ProofGate::ReviewerClassification,
        ProofGate::WithheldOrBacklogClaim,
    ];
    let reviewer_evidence = ClassifierEvidence {
        evidence_type: "reviewer_report".to_string(),
        digest: crate::self_tests::boundaries::support::sha('c'),
        summary: "human reviewer classified ambiguous surface".to_string(),
    };
    let mut failures = Vec::new();
    crate::claim_semantics::semantic::receipt::gates::required_gate_check(
        &claim,
        &ready,
        &receipt(
            gates,
            Vec::new(),
            ClassifierKind::HumanReviewer,
            Some(reviewer_evidence),
        ),
        &mut failures,
    );
    assert!(failures.is_empty());
}

#[test]
fn semantic_gate_check_rejects_missing_reviewer_and_included_backlog_substitutes() {
    let claim = json!({
        "id":"CLAIM-GATE",
        "status":"proven_static",
        "claim_ceiling_effect":"included",
        "evidence":[]
    });
    let ready = json!({"ready":false,"claim_ids":[]});
    let claim_with_runtime = json!({
        "id":"CLAIM-GATE",
        "status":"proven_static",
        "claim_ceiling_effect":"included",
        "evidence":[{"surface":"source","kind":"runtime_execution"}]
    });
    let ready_ok = json!({"ready":true,"claim_ids":["CLAIM-GATE"]});

    let mut failures = Vec::new();
    crate::claim_semantics::semantic::receipt::gates::required_gate_check(
        &claim,
        &ready,
        &receipt(
            Vec::new(),
            vec![SemanticClass::ProductUserFacingSurface],
            ClassifierKind::DeterministicBackstop,
            None,
        ),
        &mut failures,
    );
    assert!(has_fail(
        &failures,
        "semantic_classification_required_gate_unproven"
    ));

    failures.clear();
    crate::claim_semantics::semantic::receipt::gates::required_gate_check(
        &claim_with_runtime,
        &ready_ok,
        &receipt(
            vec![
                ProofGate::RuntimeExecution,
                ProofGate::ReadyForMergeReceipt,
                ProofGate::ReviewerClassification,
            ],
            vec![SemanticClass::AmbiguousNeedsReviewerClassification],
            ClassifierKind::Model,
            None,
        ),
        &mut failures,
    );
    assert!(has_fail(
        &failures,
        "semantic_classification_ambiguous_without_reviewer"
    ));

    failures.clear();
    crate::claim_semantics::semantic::receipt::gates::required_gate_check(
        &claim_with_runtime,
        &ready_ok,
        &receipt(
            vec![
                ProofGate::RuntimeExecution,
                ProofGate::ReadyForMergeReceipt,
                ProofGate::WithheldOrBacklogClaim,
            ],
            Vec::new(),
            ClassifierKind::Model,
            None,
        ),
        &mut failures,
    );
    assert!(has_fail(
        &failures,
        "semantic_classification_required_gate_unproven"
    ));
}
