use super::claims::{ClaimDefinitions, DecisionLedger};
use super::support::{REGISTRY, definitions, pass_claim, raw_digest};

#[test]
fn adopts_exact_registry_dag_and_obligation_sets() {
    let definitions = definitions();
    assert_eq!(definitions.order().len(), 14);
    assert_eq!(definitions.order()[0], "CL-SOURCE");
    assert_eq!(definitions.order()[13], "CL-COMPLETION");
    for claim_id in definitions.order() {
        let definition = definitions.definition(claim_id).expect("definition");
        let expected = definition.required_evidence.len()
            + definition.required_surface_ids.len()
            + definition.required_tool_ids.len()
            + definition.required_decision_ids.len()
            + definition.false_pass_controls.len();
        assert_eq!(
            definition.required_obligations().len(),
            expected,
            "{claim_id}"
        );
    }
}

#[test]
fn rejects_tampered_duplicate_missing_and_cyclic_registry_definitions() {
    assert!(ClaimDefinitions::adopt(REGISTRY.as_bytes(), "00").is_err());
    let duplicate = REGISTRY.replacen("\"CL-SOURCE\",", "\"CL-PACKAGE\",", 1);
    assert!(ClaimDefinitions::adopt(duplicate.as_bytes(), &raw_digest(&duplicate)).is_err());
    let missing = REGISTRY.replacen("\"CL-SOURCE\",", "\"CL-UNKNOWN\",", 1);
    assert!(ClaimDefinitions::adopt(missing.as_bytes(), &raw_digest(&missing)).is_err());
    let cycle = REGISTRY.replacen(
        "\"prerequisite_claim_ids\": [],",
        "\"prerequisite_claim_ids\": [\"CL-COMPLETION\"],",
        1,
    );
    assert!(ClaimDefinitions::adopt(cycle.as_bytes(), &raw_digest(&cycle)).is_err());
}

#[test]
fn complete_exact_obligation_sets_pass_all_fourteen_claims_in_topological_order() {
    let definitions = definitions();
    let mut ledger = super::support::semantic_model_ledger();
    for claim_id in definitions.order() {
        pass_claim(&mut ledger, &definitions, claim_id, "complete");
    }
    assert_eq!(ledger.projection(&definitions).decisions.len(), 14);
}
