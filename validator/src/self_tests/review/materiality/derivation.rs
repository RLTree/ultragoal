use super::contains;
use serde_json::{Value, json};

#[test]
fn rejects_forgery_and_substitution() {
    let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let fixture = root.join("fixtures/review-round/valid/review-round-receipt.json");
    let receipt = crate::json_boundary::read_json(&fixture).expect("review fixture");
    let anchors = crate::review::round::anchor::values::fixture_anchor_values(&root);
    let check = |value: &Value| {
        let mut out = Vec::new();
        crate::review::materiality::review_round_errors(&root, value, &anchors, &mut out);
        out.into_iter()
            .map(|failure| failure.error)
            .collect::<Vec<_>>()
    };
    let clean = check(&receipt);
    assert!(
        !contains(&clean, "materiality_decision_derivation_mismatch"),
        "{clean:?}"
    );

    let mut forged_delta = receipt.clone();
    forged_delta["materiality_gate"]["decision"] = json!("DELTA_REVIEW_ALLOWED");
    assert!(contains(
        &check(&forged_delta),
        "materiality_decision_derivation_mismatch"
    ));

    let mut missing_trigger = receipt.clone();
    missing_trigger["materiality_gate"]["material_triggers"] = json!([]);
    let got = check(&missing_trigger);
    assert!(contains(&got, "materiality_trigger_missing"));
    assert!(contains(&got, "materiality_decision_derivation_mismatch"));

    let mut tampered_trigger = receipt.clone();
    tampered_trigger["materiality_gate"]["material_triggers"] = json!(["prompt_says_delta"]);
    assert!(contains(
        &check(&tampered_trigger),
        "materiality_trigger_invalid"
    ));

    let mut placeholder = receipt.clone();
    placeholder["materiality_gate"]["anchors_checked"][0]["digest"] =
        json!("sha256:0000000000000000000000000000000000000000000000000000000000000000");
    assert!(contains(
        &check(&placeholder),
        "materiality_anchor_identity_invalid"
    ));

    let mut evidence_swap = receipt.clone();
    evidence_swap["materiality_gate"]["reviewer_registry_evidence"] =
        json!([receipt["materiality_gate"]["anchors_checked"][0].clone()]);
    assert!(contains(
        &check(&evidence_swap),
        "materiality_reviewer_registry_identity_invalid"
    ));

    let mut gate_assertion = receipt;
    gate_assertion["materiality_gate"]["deterministic_gates_run"] = json!([]);
    let got = check(&gate_assertion);
    assert!(contains(&got, "materiality_required_gate_not_run"));
    assert!(contains(&got, "materiality_decision_derivation_mismatch"));
}
