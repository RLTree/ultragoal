use super::*;
use std::collections::BTreeSet;

fn selected() -> EngineeringAdvisorySelection {
    EngineeringAdvisorySelection {
        schema_version: "EngineeringAdvisorySelection-v1".to_owned(),
        selection_id: "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            .to_owned(),
        input_fingerprint:
            "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_owned(),
        candidate_id: "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
            .to_owned(),
        context_id: "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"
            .to_owned(),
        disposition: AdvisorySelectionDisposition::AdvisorySelected,
        primary_lens: Some(AdvisoryLens::CodexTaskContract),
        supporting_lenses: Vec::new(),
        activation_reasons: vec!["typed ambiguity".to_owned()],
        assumptions: BTreeSet::from(["advisory output is proposal-only".to_owned()]),
        missing_inputs: BTreeSet::new(),
        unsupported_surfaces: BTreeSet::from(["claims".to_owned()]),
        adopting_owner: "OWN-ULTRA-ROOT".to_owned(),
        invalidation_conditions: BTreeSet::from(["candidate".to_owned(), "context".to_owned()]),
        plain_language_result: "selection".to_owned(),
        plain_language_next_action: "continue".to_owned(),
        claim_ceiling: "proposal_only_no_claim".to_owned(),
        no_claim_statement: ADVISORY_SELECTION_NO_CLAIM.to_owned(),
    }
}

#[test]
fn root_adoption_is_candidate_bound_and_cannot_promote_a_claim() {
    let selection = selected();
    let adoption = EngineeringAdvisoryAdoption::from_selection(
        &selection,
        AdvisoryAdoptionDisposition::MapAsProjection,
        BTreeSet::from(["existing plan owner".to_owned()]),
        BTreeSet::new(),
        BTreeSet::from(["focused root verification".to_owned()]),
    )
    .expect("adoption");
    assert_eq!(adoption.adopting_ultragoal_owner, "OWN-ULTRA-ROOT");
    assert_eq!(adoption.claim_ceiling, "proposal_only_no_claim");
    adoption
        .validate_against(&selection)
        .expect("valid adoption");
}

#[test]
fn forged_adoption_scope_and_stale_selection_fail_closed() {
    let selection = selected();
    let mut adoption = EngineeringAdvisoryAdoption::from_selection(
        &selection,
        AdvisoryAdoptionDisposition::Reject,
        BTreeSet::new(),
        BTreeSet::from(["unsupported artifact".to_owned()]),
        BTreeSet::from(["root decision".to_owned()]),
    )
    .expect("rejection adoption");
    adoption.claim_ceiling = "release".to_owned();
    assert!(matches!(
        adoption.validate_against(&selection),
        Err(AdvisoryError::InvalidContract(_))
    ));

    let mut stale = selection;
    stale.candidate_id =
        "sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee".to_owned();
    assert_eq!(
        adoption.validate_against(&stale),
        Err(AdvisoryError::StaleBinding)
    );
}
