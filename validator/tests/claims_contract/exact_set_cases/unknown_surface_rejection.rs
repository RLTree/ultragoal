use super::*;

#[test]
pub(crate) fn wrong_tool_surface_and_decision_ids_reject_as_unknown_and_missing() {
    let definitions = definitions();
    for kind in [
        ObligationKind::RequiredTool,
        ObligationKind::RequiredSurface,
        ObligationKind::RequiredDecision,
    ] {
        let claim_id = "CL-RELEASE";
        let mut ledger = super::super::scenario::semantic_model_ledger();
        pass_before(&mut ledger, &definitions, claim_id, "wrong-binding-prior");
        let mut observations = observations_for(&definitions, claim_id, "wrong-binding");
        let position = observations
            .iter()
            .position(|item| item.envelope().obligation.kind == kind)
            .expect("obligation kind");
        let mut envelope = observations.remove(position).envelope().clone();
        let old = envelope.obligation.id.clone();
        let wrong = format!("wrong-{old}");
        envelope.obligation.id = wrong.clone();
        let result = envelope.result.result_digest().to_owned();
        envelope.outputs.remove(&old);
        envelope.outputs.insert(wrong.clone(), result);
        envelope.inputs.remove(&old);
        envelope.environment_and_tools.remove(&old);
        match kind {
            ObligationKind::RequiredTool => {
                envelope
                    .environment_and_tools
                    .insert(wrong, digest("wrong-tool"));
            }
            _ => {
                envelope.inputs.insert(wrong, digest("wrong-input"));
            }
        }
        observations.push(envelope.observe().expect("wrong binding"));
        let decision = decide(&mut ledger, &definitions, claim_id, observations);
        assert_eq!(decision.status, DecisionStatus::Rejected);
        assert!(
            decision
                .reasons
                .iter()
                .any(|reason| reason.contains("unknown"))
        );
        assert!(
            decision
                .reasons
                .iter()
                .any(|reason| reason.contains("missing"))
        );
    }
}

#[test]
pub(crate) fn self_authored_material_scorer_and_reviewer_only_proof_reject() {
    let definitions = definitions();
    let claim_id = "CL-PACKAGE";
    for case in ["self", "material", "reviewer-only"] {
        let mut ledger = super::super::scenario::semantic_model_ledger();
        pass_before(
            &mut ledger,
            &definitions,
            claim_id,
            &format!("{case}-prior"),
        );
        let mut observations = observations_for(&definitions, claim_id, case);
        let mut envelope = observations.remove(0).envelope().clone();
        match case {
            "self" => envelope.producer.actor_id = reviewer(&definitions, claim_id).actor_id,
            "material" => {
                envelope.producer.roles.insert(ActorRole::MaterialScorer);
            }
            "reviewer-only" => {
                envelope.producer = actor("reviewer-only", &[ActorRole::IndependentReviewer]);
            }
            _ => unreachable!(),
        }
        observations.push(envelope.observe().expect("actor mutation"));
        assert_eq!(
            decide(&mut ledger, &definitions, claim_id, observations).status,
            DecisionStatus::Rejected
        );
    }
}
