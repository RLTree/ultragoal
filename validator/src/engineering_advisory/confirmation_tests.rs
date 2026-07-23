use super::*;
use crate::orchestration::{
    Actor, Binding, CanonicalPath, LeaseSpec, OwnedScope, Principal, ReviewDecision, ReviewRecord,
    SafetyClass, WorkPackage,
};
use std::collections::{BTreeMap, BTreeSet};

const CANDIDATE: &str = "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const CONTEXT: &str = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

#[test]
fn task_packet_rejects_narrowed_or_mutated_scope_and_added_verification() {
    let (package, lease) = task_fixture();
    let packet = TaskEvidencePacket::from_work_package(&package, &lease).unwrap();

    let mut narrowed = packet.clone();
    narrowed.owned_scope.paths.clear();
    assert_eq!(
        narrowed.validate_against(&package, &lease),
        Err(AdvisoryError::PermissionWidening)
    );

    let mut mutated = packet.clone();
    mutated.owned_scope.paths = BTreeSet::from([CanonicalPath::parse("src/other.rs").unwrap()]);
    assert_eq!(
        mutated.validate_against(&package, &lease),
        Err(AdvisoryError::PermissionWidening)
    );

    let mut added_verification = packet;
    added_verification
        .required_verification
        .insert("extra-check".to_owned());
    assert_eq!(
        added_verification.validate_against(&package, &lease),
        Err(AdvisoryError::MissingVerification)
    );
}

#[test]
fn deserialized_review_verdict_rejects_release_and_completion_ceiling() {
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
    let verdict = ReviewVerdict::from_review(
        &review,
        &ReviewMaterialityOutput {
            binding,
            review_id: review.review_id().unwrap(),
            claim_ceiling: BTreeMap::from([(
                "CL-SOURCE".to_owned(),
                BTreeSet::from(["source".to_owned()]),
            )]),
            unverifiable_claims: BTreeSet::new(),
            rerun_command_id: "check-source".to_owned(),
            material: true,
        },
    )
    .unwrap();

    for dimension in ["release", "completion"] {
        let mut encoded = serde_json::to_value(&verdict).unwrap();
        encoded["claim_ceiling"] = serde_json::json!({"CL-SOURCE": [dimension]});
        let forged: ReviewVerdict = serde_json::from_value(encoded).unwrap();
        assert_eq!(forged.validate(), Err(AdvisoryError::ClaimPromotion));
    }
}

fn task_fixture() -> (WorkPackage, LeaseSpec) {
    let path = CanonicalPath::parse("src/lib.rs").unwrap();
    let scope = OwnedScope {
        paths: BTreeSet::from([path]),
        ..OwnedScope::default()
    };
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
        binding: Binding::new(CONTEXT, CANDIDATE).unwrap(),
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
