use super::*;
use crate::orchestration::{
    Actor, Binding, CanonicalPath, LeaseRegistry, LeaseSpec, OwnedScope, PrerequisiteEvidence,
    Principal, ReviewDecision, ReviewRecord, SafetyClass, ScopePolicy, WorkPackage,
};
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
    let policy = task_policy(&package);
    let registry = active_registry(&lease, &policy);
    let packet =
        TaskEvidencePacket::from_work_package(&package, &lease, &registry, &policy, &lease.binding)
            .unwrap();
    assert_eq!(packet.schema_version, "TaskEvidencePacket-v1");
    assert!(
        packet
            .validate_against(&package, &lease, &registry, &policy, &lease.binding)
            .is_ok()
    );
    assert_eq!(packet.required_tools, package.required_tools);
    assert_eq!(packet.prerequisite_evidence, lease.prerequisite_evidence);
    assert_eq!(packet.cancellation_authority, "root_lease_revocation");

    let mut widened = packet.clone();
    widened
        .owned_scope
        .paths
        .insert(CanonicalPath::parse("src/other.rs").unwrap());
    assert_eq!(
        widened.validate_against(&package, &lease, &registry, &policy, &lease.binding),
        Err(AdvisoryError::PermissionWidening)
    );

    let mut missing = packet;
    missing.required_verification.clear();
    assert_eq!(
        missing.validate_against(&package, &lease, &registry, &policy, &lease.binding),
        Err(AdvisoryError::MissingVerification)
    );

    let mut stale_budget =
        TaskEvidencePacket::from_work_package(&package, &lease, &registry, &policy, &lease.binding)
            .unwrap();
    stale_budget.max_retries += 1;
    assert_eq!(
        stale_budget.validate_against(&package, &lease, &registry, &policy, &lease.binding),
        Err(AdvisoryError::StaleBinding)
    );

    let mut invalid_lease = lease.clone();
    invalid_lease.heartbeat_deadline_tick = invalid_lease.issued_tick;
    assert_eq!(
        TaskEvidencePacket::from_work_package(
            &package,
            &invalid_lease,
            &registry,
            &policy,
            &lease.binding
        ),
        Err(AdvisoryError::PermissionWidening)
    );
    assert_eq!(
        TaskEvidencePacket::from_work_package(
            &package,
            &lease,
            &LeaseRegistry::default(),
            &policy,
            &lease.binding
        ),
        Err(AdvisoryError::StaleBinding)
    );
}

#[test]
fn structured_review_findings_bind_every_root_finding_code() {
    let binding = Binding::new(CONTEXT, CANDIDATE).unwrap();
    let review = ReviewRecord {
        reviewer: "/reviewer".to_owned(),
        worker: "/worker".to_owned(),
        binding: binding.clone(),
        result_id: digest('b'),
        result_commitment_id: digest('c'),
        decision: ReviewDecision::Rework,
        reproduced_commands: BTreeSet::from(["check-source".to_owned()]),
        finding_codes: BTreeSet::from(["authority-gap".to_owned()]),
    };
    let finding = ReviewFinding {
        code: "authority-gap".to_owned(),
        severity: ReviewFindingSeverity::P0,
        evidence: BTreeSet::from(["source:authority-gap".to_owned()]),
        suggested_action: Some("repair the existing authority boundary".to_owned()),
    };
    let verdict = ReviewVerdict::from_review(
        &review,
        &ReviewMaterialityOutput {
            binding,
            review_id: review.review_id().unwrap(),
            findings: BTreeMap::from([("authority-gap".to_owned(), finding)]),
            claim_ceiling: BTreeMap::from([(
                String::from("CL-SOURCE"),
                BTreeSet::from([String::from("source")]),
            )]),
            unverifiable_claims: BTreeSet::new(),
            rerun_command_id: "check-source".to_owned(),
            material: true,
        },
    )
    .unwrap();
    assert!(verdict.validate_against(&review).is_ok());
    let mut omitted = verdict;
    omitted.findings.clear();
    assert_eq!(
        omitted.validate(),
        Err(AdvisoryError::InvalidReview(
            "structured review findings do not match the root review record"
        ))
    );
}

fn task_policy(package: &WorkPackage) -> ScopePolicy {
    ScopePolicy {
        allowed_read_paths: package.read_paths.clone(),
        allowed_paths: package.owned_scope.paths.clone(),
        ..ScopePolicy::default()
    }
}

fn active_registry(lease: &LeaseSpec, policy: &ScopePolicy) -> LeaseRegistry {
    let mut registry = LeaseRegistry::default();
    registry
        .grant(lease.clone(), policy, &lease.binding)
        .unwrap();
    registry
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
        dependencies: BTreeSet::from(["node-context".to_owned()]),
        required_tools: BTreeSet::from(["rustfmt".to_owned()]),
        safety_class: SafetyClass::IsolatedWorkspaceWrite,
        read_paths: BTreeSet::from([CanonicalPath::parse("tests").unwrap()]),
        owned_scope: scope.clone(),
        prerequisites: BTreeSet::from(["current-candidate".to_owned()]),
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
        prerequisite_evidence: PrerequisiteEvidence {
            dependency_nodes: BTreeMap::from([("node-context".to_owned(), digest('d'))]),
            required_tools: BTreeMap::from([("rustfmt".to_owned(), digest('e'))]),
            prerequisites: BTreeMap::from([("current-candidate".to_owned(), digest('f'))]),
        },
        issued_tick: 1,
        heartbeat_deadline_tick: 2,
        max_retries: 1,
    };
    (package, lease)
}

fn digest(fill: char) -> String {
    format!("sha256:{}", fill.to_string().repeat(64))
}
