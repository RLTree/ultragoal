use super::*;
use crate::context::EffectClass;
use crate::orchestration::{
    Actor, Binding, CanonicalPath, LeaseSpec, OwnedScope, Principal, ReviewDecision, ReviewRecord,
    SafetyClass, WorkPackage,
};
use crate::state::{Repair, RepairTarget, RepairTargetKind};
use std::collections::{BTreeMap, BTreeSet};

const CANDIDATE: &str = "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const CONTEXT: &str = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

#[test]
fn verification_contract_requires_root_fields_and_rejects_universal_tdd() {
    let empty = VerificationModeProposal::new(VerificationModeProposalInput {
        candidate_id: CANDIDATE.to_owned(),
        risk: String::new(),
        failure_model: "semantic mutation".to_owned(),
        oracle: "oracle".to_owned(),
        truth_surface: "source".to_owned(),
        selected_modes: BTreeSet::from(["targeted".to_owned()]),
        rejected_modes: BTreeSet::new(),
        required_evidence: BTreeSet::from(["command".to_owned()]),
        claim_ceiling: BTreeMap::from([(
            String::from("CL-SOURCE"),
            BTreeSet::from([String::from("source")]),
        )]),
        invalidation_trigger: "source changed".to_owned(),
    });
    assert!(matches!(empty, Err(AdvisoryError::InvalidContract(_))));
    let tdd = VerificationModeProposal::new(VerificationModeProposalInput {
        candidate_id: CANDIDATE.to_owned(),
        risk: "high".to_owned(),
        failure_model: "semantic mutation".to_owned(),
        oracle: "oracle".to_owned(),
        truth_surface: "source".to_owned(),
        selected_modes: BTreeSet::from(["tdd".to_owned()]),
        rejected_modes: BTreeSet::new(),
        required_evidence: BTreeSet::from(["command".to_owned()]),
        claim_ceiling: BTreeMap::from([(
            String::from("CL-SOURCE"),
            BTreeSet::from([String::from("source")]),
        )]),
        invalidation_trigger: "source changed".to_owned(),
    });
    assert!(matches!(tdd, Err(AdvisoryError::InvalidContract(_))));

    let proposal = VerificationModeProposal::new(VerificationModeProposalInput {
        candidate_id: CANDIDATE.to_owned(),
        risk: "high".to_owned(),
        failure_model: "semantic mutation".to_owned(),
        oracle: "oracle".to_owned(),
        truth_surface: "source".to_owned(),
        selected_modes: BTreeSet::from(["targeted".to_owned()]),
        rejected_modes: BTreeSet::new(),
        required_evidence: BTreeSet::from(["command".to_owned()]),
        claim_ceiling: BTreeMap::from([(
            String::from("CL-SOURCE"),
            BTreeSet::from([String::from("source")]),
        )]),
        invalidation_trigger: "source changed".to_owned(),
    })
    .unwrap();
    let forged = serde_json::from_value::<VerificationModeContract>(serde_json::json!({
        "schema_version": "VerificationModeContract-v1",
        "contract_id": "sha256:1111111111111111111111111111111111111111111111111111111111111111",
        "candidate_id": CANDIDATE,
        "risk": "high",
        "failure_model": "semantic mutation",
        "oracle": "oracle",
        "truth_surface": "source",
        "selected_modes": ["targeted"],
        "rejected_modes": [],
        "required_evidence": ["command"],
        "claim_ceiling": {"CL-SOURCE": ["source"]},
        "invalidation_trigger": "source changed",
        "issuer": "ultragoal-root",
        "authority_id": "forged"
    }))
    .unwrap();
    assert!(forged.validate().is_err());
    let sealed = VerificationModeContract::seal_from_root(proposal, "authority").unwrap();
    assert!(sealed.validate().is_ok());
}

#[test]
fn task_packet_is_proposal_only_and_rejects_widening_or_missing_verification() {
    let (package, lease) = task_fixture();
    let packet = TaskEvidencePacket::from_work_package(&package, &lease).unwrap();
    assert_eq!(packet.schema_version, "TaskEvidencePacket-v1");
    assert!(packet.validate_against(&package, &lease).is_ok());

    let mut widened = packet.clone();
    widened
        .owned_scope
        .paths
        .insert(CanonicalPath::parse("src/other.rs").unwrap());
    assert_eq!(
        widened.validate_against(&package, &lease),
        Err(AdvisoryError::PermissionWidening)
    );

    let mut missing = packet;
    missing.required_verification.clear();
    assert_eq!(
        missing.validate_against(&package, &lease),
        Err(AdvisoryError::MissingVerification)
    );
}

#[test]
fn review_verdict_preserves_independence_and_cannot_promote() {
    let binding = Binding::new(CONTEXT, CANDIDATE).unwrap();
    let review = ReviewRecord {
        reviewer: "/reviewer".to_owned(),
        worker: "/worker".to_owned(),
        binding: binding.clone(),
        result_id: digest('b'),
        result_commitment_id: digest('c'),
        decision: ReviewDecision::Pass,
        reproduced_commands: BTreeSet::from(["check-source".to_owned()]),
        finding_codes: BTreeSet::new(),
    };
    let materiality = ReviewMaterialityOutput {
        binding,
        review_id: review.review_id().unwrap(),
        claim_ceiling: BTreeMap::from([(
            String::from("CL-SOURCE"),
            BTreeSet::from([String::from("source")]),
        )]),
        unverifiable_claims: BTreeSet::new(),
        rerun_command_id: "check-source".to_owned(),
        material: true,
    };
    let verdict = ReviewVerdict::from_review(&review, &materiality).unwrap();
    assert!(!verdict.reviewer_can_promote);
    assert!(verdict.validate().is_ok());
    let mut forged = verdict;
    forged.reviewer_can_promote = true;
    assert_eq!(
        forged.validate(),
        Err(AdvisoryError::InvalidReview(
            "review verdict authority boundary violated"
        ))
    );
}

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
    let continued =
        decide_semantic_repair(&previous, &new_evidence, &repair, RepairCircuitRoute::Retry)
            .unwrap();
    assert_eq!(continued.decision, SemanticRepairDecision::Continue);
    let stopped =
        decide_semantic_repair(&previous, &previous, &repair, RepairCircuitRoute::Retry).unwrap();
    assert_eq!(stopped.decision, SemanticRepairDecision::Stop);
    let mut proxy = new_evidence;
    proxy.proxy_signal = true;
    assert_eq!(
        decide_semantic_repair(&previous, &proxy, &repair, RepairCircuitRoute::Improvement),
        Err(AdvisoryError::ProxyGaming)
    );
}

fn task_fixture() -> (WorkPackage, LeaseSpec) {
    let path = CanonicalPath::parse("src/lib.rs").unwrap();
    let scope = OwnedScope {
        paths: BTreeSet::from([path.clone()]),
        ..OwnedScope::default()
    };
    let binding = Binding::new(CONTEXT, CANDIDATE).unwrap();
    let package = WorkPackage {
        node_id: "node-source".to_owned(),
        dependencies: BTreeSet::new(),
        required_tools: BTreeSet::new(),
        safety_class: SafetyClass::IsolatedWorkspaceWrite,
        read_paths: BTreeSet::from([CanonicalPath::parse("tests").unwrap()]),
        owned_scope: scope.clone(),
        prerequisites: BTreeSet::new(),
        outputs: BTreeSet::from(["source".to_owned()]),
        acceptance: BTreeSet::from(["semantic-check".to_owned()]),
        claim_effect: "source".to_owned(),
    };
    let lease = LeaseSpec {
        lease_id: "lease-source".to_owned(),
        run_id: "run-source".to_owned(),
        node_id: package.node_id.clone(),
        principal: Principal::Worker,
        owner: Actor::parse("/worker").unwrap(),
        binding,
        safety_class: package.safety_class,
        read_paths: package.read_paths.clone(),
        owned_scope: scope,
        prerequisite_evidence: Default::default(),
        issued_tick: 1,
        heartbeat_deadline_tick: 2,
        max_retries: 1,
    };
    (package, lease)
}

fn digest(fill: char) -> String {
    format!("sha256:{}", fill.to_string().repeat(64))
}
