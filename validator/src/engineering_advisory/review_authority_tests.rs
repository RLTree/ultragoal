use super::*;
use crate::orchestration::{Binding, ReviewDecision, ReviewRecord};
use std::collections::{BTreeMap, BTreeSet};

const CANDIDATE: &str = "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const CONTEXT: &str = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

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
        findings: BTreeMap::new(),
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
    assert!(verdict.validate_against(&review).is_ok());
    let mut forged = verdict;
    forged.reviewer_can_promote = true;
    assert_eq!(
        forged.validate(),
        Err(AdvisoryError::InvalidReview(
            "review verdict authority boundary violated"
        ))
    );

    let mut stale_review = review;
    stale_review.binding = Binding::new(CONTEXT, &digest('d')).unwrap();
    let current = ReviewVerdict::from_review(
        &ReviewRecord {
            binding: Binding::new(CONTEXT, CANDIDATE).unwrap(),
            ..stale_review.clone()
        },
        &materiality,
    )
    .unwrap();
    assert_eq!(
        current.validate_against(&stale_review),
        Err(AdvisoryError::StaleBinding)
    );
}

fn digest(fill: char) -> String {
    format!("sha256:{}", fill.to_string().repeat(64))
}
