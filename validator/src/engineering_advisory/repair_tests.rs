use super::*;
use crate::context::EffectClass;
use crate::orchestration::Binding;
use crate::state::{
    AuthorityRequirement, CurrentBehaviorDisposition, HostGoalObservation, NextAction,
    NextActionKind, ProductGoalState, ProductState, Repair, RepairTarget, RepairTargetKind,
    RoutineObservationWindow, RuntimeMetadata,
};
use std::collections::BTreeSet;

const CANDIDATE: &str = "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const CONTEXT: &str = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

#[test]
fn semantic_repair_requires_new_semantic_signal_and_rejects_proxies() {
    let repair = Repair {
        repair_id: "repair-source".to_owned(),
        target: RepairTarget {
            kind: RepairTargetKind::Source,
            id: "source".to_owned(),
        },
        summary: "repair source".to_owned(),
        effect: EffectClass::Read,
        authority: crate::state::AuthorityRequirement::Root,
        rerun_command_id: "inspect-json".to_owned(),
        authority_decision: None,
        invalidates_evidence: BTreeSet::new(),
        projected_ceiling_after_reverification: Vec::new(),
    };
    let state = state_with_repair(repair.clone());
    let binding = Binding::new(CONTEXT, CANDIDATE).unwrap();
    let previous = SemanticRepairObservation {
        candidate_id: CANDIDATE.to_owned(),
        hypothesis: "wrong parser".to_owned(),
        mechanism: "branch".to_owned(),
        evidence: BTreeSet::from(["e1".to_owned()]),
        ambiguous: false,
        proxy_signal: false,
    };
    let mut new_evidence = previous.clone();
    new_evidence.evidence.insert("e2".to_owned());
    let continued = decide_semantic_repair(
        &state,
        &binding,
        &previous,
        &new_evidence,
        &repair.repair_id,
        RepairCircuitRoute::Retry,
    )
    .unwrap();
    assert_eq!(continued.decision, SemanticRepairDecision::Continue);
    let stopped = decide_semantic_repair(
        &state,
        &binding,
        &previous,
        &previous,
        &repair.repair_id,
        RepairCircuitRoute::Retry,
    )
    .unwrap();
    assert_eq!(stopped.decision, SemanticRepairDecision::Stop);
    assert!(
        stopped
            .validate_against(
                &state,
                &binding,
                &previous,
                &previous,
                &repair.repair_id,
                RepairCircuitRoute::Retry,
            )
            .is_ok()
    );
    let mut forged = stopped;
    forged.decision = SemanticRepairDecision::Continue;
    assert_eq!(
        forged.validate_against(
            &state,
            &binding,
            &previous,
            &previous,
            &repair.repair_id,
            RepairCircuitRoute::Retry,
        ),
        Err(AdvisoryError::InvalidContract(
            "repair circuit does not match the current evidence"
        ))
    );
    let mut proxy = new_evidence;
    proxy.proxy_signal = true;
    assert_eq!(
        decide_semantic_repair(
            &state,
            &binding,
            &previous,
            &proxy,
            &repair.repair_id,
            RepairCircuitRoute::Improvement,
        ),
        Err(AdvisoryError::ProxyGaming)
    );

    let stale = Binding::new(CONTEXT, &format!("sha256:{}", "d".repeat(64))).unwrap();
    assert_eq!(
        decide_semantic_repair(
            &state,
            &stale,
            &previous,
            &previous,
            &repair.repair_id,
            RepairCircuitRoute::Retry,
        ),
        Err(AdvisoryError::CandidateMismatch)
    );
    assert_eq!(
        decide_semantic_repair(
            &state,
            &binding,
            &previous,
            &previous,
            "forged-repair",
            RepairCircuitRoute::Retry,
        ),
        Err(AdvisoryError::InvalidContract(
            "repair is not present in the current product state"
        ))
    );
}

fn state_with_repair(repair: Repair) -> ProductState {
    ProductState {
        schema_version: "ProductState-v1",
        state_id: "sha256:state".to_owned(),
        context_id: CONTEXT.to_owned(),
        authority_catalog_id: "sha256:authority".to_owned(),
        candidate_id: CANDIDATE.to_owned(),
        dependency_action_catalog_id: "sha256:catalog".to_owned(),
        product_goal: ProductGoalState::Operating,
        current_behavior: CurrentBehaviorDisposition::ChangeRequired,
        host_goal: HostGoalObservation::default(),
        runtime_metadata: RuntimeMetadata::default(),
        findings: Vec::new(),
        repairs: vec![repair],
        claim_ceilings: Vec::new(),
        next_action: NextAction {
            kind: NextActionKind::NoOp,
            action_id: "no-op".to_owned(),
            priority: u32::MAX,
            repair_id: None,
            effect: EffectClass::Read,
            authority: AuthorityRequirement::None,
            command_id: None,
            exact_command: None,
            authority_request: None,
            no_legal_route: None,
            priority_class: None,
            active_transition: None,
            brief_digest: None,
            active_trigger_ids: Vec::new(),
            parked_trigger_ids: Vec::new(),
            verification_mode: None,
            selection_rule: "test-only-current-state",
        },
        routine_observations: Vec::new(),
        routine_observation_window: RoutineObservationWindow::NotQueried,
    }
}
