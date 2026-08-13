use super::*;
use std::collections::BTreeSet;

const CANDIDATE: &str = "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const CONTEXT: &str = "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
const CONFIG: &str = "sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";

fn binding(selection: &EngineeringAdvisorySelection) -> EngineeringAdvisoryAdoptionBinding {
    let profile = crate::plugin_product::skill_catalog::coinstall_profile(
        crate::plugin_product::skill_catalog::AgenticAdvisoryStage::Core,
        CANDIDATE,
        CONFIG,
    );
    EngineeringAdvisoryAdoptionBinding::exact(
        &selection.selection_id,
        CANDIDATE,
        CONTEXT,
        profile.source_profile_digest,
    )
    .expect("exact adoption binding")
}

fn remint_selection_id(selection: &mut EngineeringAdvisorySelection) {
    selection.selection_id = super::selection_response::selection_identity(selection);
}

fn selected() -> EngineeringAdvisorySelection {
    let profile = crate::plugin_product::skill_catalog::coinstall_profile(
        crate::plugin_product::skill_catalog::AgenticAdvisoryStage::Core,
        CANDIDATE,
        CONFIG,
    );
    let mut selection = EngineeringAdvisorySelection {
        schema_version: "EngineeringAdvisorySelection-v2".to_owned(),
        selection_id: String::new(),
        input_fingerprint:
            "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_owned(),
        candidate_id: CANDIDATE.to_owned(),
        context_id: CONTEXT.to_owned(),
        pack_set_digest: crate::plugin_product::engineering_advisory::AGENTIC_PACK_SET_DIGEST
            .to_owned(),
        profile_digest: profile.source_profile_digest,
        selector_version: "EngineeringAdvisorySelector-v2".to_owned(),
        agentic_candidate: Some(
            crate::plugin_product::engineering_advisory::exact_agentic_candidate_binding(),
        ),
        disposition: AdvisorySelectionDisposition::AdvisorySelected,
        primary_lens: Some(AdvisoryLens::CodexTaskContract),
        qualified_primary_skill: Some("agentic-engineering:codex-task-contract".to_owned()),
        supporting_lenses: Vec::new(),
        qualified_supporting_skills: Vec::new(),
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
    };
    remint_selection_id(&mut selection);
    selection
}

#[test]
fn root_adoption_is_candidate_bound_and_cannot_promote_a_claim() {
    let selection = selected();
    let adoption = EngineeringAdvisoryAdoption::from_selection(
        &selection,
        &binding(&selection),
        AdvisoryAdoptionDisposition::MapAsProjection,
        BTreeSet::from(["existing plan owner".to_owned()]),
        BTreeSet::new(),
        BTreeSet::from(["focused root verification".to_owned()]),
    )
    .expect("adoption");
    assert_eq!(adoption.adopting_ultragoal_owner, "OWN-ULTRA-ROOT");
    assert_eq!(adoption.schema_version, ADVISORY_ADOPTION_SCHEMA);
    assert_eq!(adoption.claim_ceiling, "proposal_only_no_claim");
    adoption
        .validate_against(&selection, &binding(&selection))
        .expect("valid adoption");
}

#[test]
fn forged_adoption_scope_and_stale_selection_fail_closed() {
    let selection = selected();
    let expected = binding(&selection);
    let mut adoption = EngineeringAdvisoryAdoption::from_selection(
        &selection,
        &expected,
        AdvisoryAdoptionDisposition::Reject,
        BTreeSet::new(),
        BTreeSet::from(["unsupported artifact".to_owned()]),
        BTreeSet::from(["root decision".to_owned()]),
    )
    .expect("rejection adoption");
    adoption.claim_ceiling = "release".to_owned();
    assert!(matches!(
        adoption.validate_against(&selection, &expected),
        Err(AdvisoryError::InvalidContract(_))
    ));

    let mut stale = selection;
    stale.candidate_id =
        "sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee".to_owned();
    assert_eq!(
        adoption.validate_against(&stale, &expected),
        Err(AdvisoryError::StaleBinding)
    );
}

#[test]
fn legacy_adoption_wire_shape_is_rejected() {
    let selection = selected();
    let mut adoption = EngineeringAdvisoryAdoption::from_selection(
        &selection,
        &binding(&selection),
        AdvisoryAdoptionDisposition::Reuse,
        BTreeSet::from(["existing owner".to_owned()]),
        BTreeSet::new(),
        BTreeSet::from(["root check".to_owned()]),
    )
    .expect("adoption");
    adoption.schema_version = "EngineeringAdvisoryAdoption-v1".to_owned();
    assert!(matches!(
        adoption.validate_against(&selection, &binding(&selection)),
        Err(AdvisoryError::InvalidContract(_))
    ));

    let mut unknown = EngineeringAdvisoryAdoption::from_selection(
        &selection,
        &binding(&selection),
        AdvisoryAdoptionDisposition::Reuse,
        BTreeSet::from(["existing owner".to_owned()]),
        BTreeSet::new(),
        BTreeSet::from(["root check".to_owned()]),
    )
    .expect("adoption");
    unknown.schema_version = "EngineeringAdvisoryAdoption-v3".to_owned();
    assert!(matches!(
        unknown.validate_against(&selection, &binding(&selection)),
        Err(AdvisoryError::InvalidContract(_))
    ));

    let mut legacy_domain = EngineeringAdvisoryAdoption::from_selection(
        &selection,
        &binding(&selection),
        AdvisoryAdoptionDisposition::Reuse,
        BTreeSet::from(["existing owner".to_owned()]),
        BTreeSet::new(),
        BTreeSet::from(["root check".to_owned()]),
    )
    .expect("adoption");
    legacy_domain.proposal_digest =
        crate::digest::bytes(&serde_json::to_vec(&selection).expect("legacy proposal serializes"));
    assert!(matches!(
        legacy_domain.validate_against(&selection, &binding(&selection)),
        Err(AdvisoryError::InvalidContract(_))
    ));
}

#[test]
fn adoption_v2_rejects_unknown_or_missing_new_wire_fields_one_axis_at_a_time() {
    let selection = selected();
    let adoption = EngineeringAdvisoryAdoption::from_selection(
        &selection,
        &binding(&selection),
        AdvisoryAdoptionDisposition::Reuse,
        BTreeSet::from(["existing owner".to_owned()]),
        BTreeSet::new(),
        BTreeSet::from(["root check".to_owned()]),
    )
    .expect("adoption");
    let exact = serde_json::to_value(&adoption).expect("adoption serializes");

    for field in [
        "pack_set_digest",
        "profile_digest",
        "selector_version",
        "agentic_candidate",
    ] {
        let mut missing = exact.clone();
        missing
            .as_object_mut()
            .expect("adoption object")
            .remove(field);
        assert!(
            serde_json::from_value::<EngineeringAdvisoryAdoption>(missing).is_err(),
            "missing {field} must fail"
        );
    }

    let mut unknown = exact;
    unknown
        .as_object_mut()
        .expect("adoption object")
        .insert("unexpected_binding".to_owned(), serde_json::json!(true));
    assert!(serde_json::from_value::<EngineeringAdvisoryAdoption>(unknown).is_err());
}

#[test]
fn legacy_or_unqualified_selection_cannot_enter_adoption() {
    let mut legacy = selected();
    legacy.schema_version = "EngineeringAdvisorySelection-v1".to_owned();
    assert!(matches!(
        EngineeringAdvisoryAdoption::from_selection(
            &legacy,
            &binding(&legacy),
            AdvisoryAdoptionDisposition::Reuse,
            BTreeSet::from(["existing owner".to_owned()]),
            BTreeSet::new(),
            BTreeSet::from(["root check".to_owned()]),
        ),
        Err(AdvisoryError::StaleBinding)
    ));

    let mut unqualified = selected();
    unqualified.qualified_primary_skill = Some("codex-task-contract".to_owned());
    assert!(matches!(
        EngineeringAdvisoryAdoption::from_selection(
            &unqualified,
            &binding(&unqualified),
            AdvisoryAdoptionDisposition::Reuse,
            BTreeSet::from(["existing owner".to_owned()]),
            BTreeSet::new(),
            BTreeSet::from(["root check".to_owned()]),
        ),
        Err(AdvisoryError::StaleBinding)
    ));

    let mut forged_id = selected();
    forged_id.selection_id =
        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_owned();
    assert!(matches!(
        EngineeringAdvisoryAdoption::from_selection(
            &forged_id,
            &binding(&forged_id),
            AdvisoryAdoptionDisposition::Reuse,
            BTreeSet::from(["existing owner".to_owned()]),
            BTreeSet::new(),
            BTreeSet::from(["root check".to_owned()]),
        ),
        Err(AdvisoryError::StaleBinding)
    ));

    let mut wrong_pack = selected();
    wrong_pack.pack_set_digest =
        "sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".to_owned();
    remint_selection_id(&mut wrong_pack);
    assert!(matches!(
        EngineeringAdvisoryAdoption::from_selection(
            &wrong_pack,
            &binding(&wrong_pack),
            AdvisoryAdoptionDisposition::Reuse,
            BTreeSet::from(["existing owner".to_owned()]),
            BTreeSet::new(),
            BTreeSet::from(["root check".to_owned()]),
        ),
        Err(AdvisoryError::StaleBinding)
    ));
}

#[test]
fn full_selection_payload_is_selection_id_bound() {
    for field in [
        "activation_reasons",
        "assumptions",
        "unsupported_surfaces",
        "invalidation_conditions",
        "plain_language_result",
    ] {
        let original = selected();
        let mut changed = original.clone();
        match field {
            "activation_reasons" => changed.activation_reasons.clear(),
            "assumptions" => changed.assumptions.clear(),
            "unsupported_surfaces" => changed.unsupported_surfaces.clear(),
            "invalidation_conditions" => changed.invalidation_conditions.clear(),
            "plain_language_result" => changed.plain_language_result = "misleading".to_owned(),
            _ => unreachable!(),
        }
        assert!(matches!(
            EngineeringAdvisoryAdoption::from_selection(
                &changed,
                &binding(&original),
                AdvisoryAdoptionDisposition::Reuse,
                BTreeSet::from(["existing owner".to_owned()]),
                BTreeSet::new(),
                BTreeSet::from(["root check".to_owned()]),
            ),
            Err(AdvisoryError::StaleBinding)
        ));

        remint_selection_id(&mut changed);
        assert!(matches!(
            EngineeringAdvisoryAdoption::from_selection(
                &changed,
                &binding(&original),
                AdvisoryAdoptionDisposition::Reuse,
                BTreeSet::from(["existing owner".to_owned()]),
                BTreeSet::new(),
                BTreeSet::from(["root check".to_owned()]),
            ),
            Err(AdvisoryError::StaleBinding)
        ));
    }
}

#[test]
fn candidate_context_and_specific_stage_profile_are_immutable_adoption_bindings() {
    let mut wrong_candidate = selected();
    wrong_candidate.candidate_id =
        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_owned();
    remint_selection_id(&mut wrong_candidate);
    let mut wrong_context = selected();
    wrong_context.context_id =
        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_owned();
    remint_selection_id(&mut wrong_context);
    for changed in [&wrong_candidate, &wrong_context] {
        assert!(matches!(
            EngineeringAdvisoryAdoption::from_selection(
                changed,
                &binding(changed),
                AdvisoryAdoptionDisposition::Reuse,
                BTreeSet::from(["existing owner".to_owned()]),
                BTreeSet::new(),
                BTreeSet::from(["root check".to_owned()]),
            ),
            Err(AdvisoryError::StaleBinding)
        ));
    }

    let mut wrong_stage = selected();
    wrong_stage.profile_digest = crate::plugin_product::skill_catalog::coinstall_profile(
        crate::plugin_product::skill_catalog::AgenticAdvisoryStage::Lifecycle,
        &wrong_stage.candidate_id,
        "sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
    )
    .source_profile_digest;
    remint_selection_id(&mut wrong_stage);
    assert!(matches!(
        EngineeringAdvisoryAdoption::from_selection(
            &wrong_stage,
            &binding(&wrong_stage),
            AdvisoryAdoptionDisposition::Reuse,
            BTreeSet::from(["existing owner".to_owned()]),
            BTreeSet::new(),
            BTreeSet::from(["root check".to_owned()]),
        ),
        Err(AdvisoryError::StaleBinding)
    ));

    let mut wrong_selector = selected();
    wrong_selector.selector_version = "EngineeringAdvisorySelector-v3".to_owned();
    remint_selection_id(&mut wrong_selector);
    assert!(matches!(
        EngineeringAdvisoryAdoption::from_selection(
            &wrong_selector,
            &binding(&wrong_selector),
            AdvisoryAdoptionDisposition::Reuse,
            BTreeSet::from(["existing owner".to_owned()]),
            BTreeSet::new(),
            BTreeSet::from(["root check".to_owned()]),
        ),
        Err(AdvisoryError::StaleBinding)
    ));

    for field in [
        "source_commit",
        "source_tree",
        "manifest_digest",
        "package_digest",
    ] {
        let mut wrong_candidate_binding = selected();
        let candidate = wrong_candidate_binding
            .agentic_candidate
            .as_mut()
            .expect("candidate binding");
        match field {
            "source_commit" => candidate.source_commit = "a".repeat(40),
            "source_tree" => candidate.source_tree = "b".repeat(40),
            "manifest_digest" => {
                *candidate
                    .manifest_digests
                    .get_mut("agentic-engineering")
                    .expect("base manifest") =
                    "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                        .to_owned();
            }
            "package_digest" => {
                *candidate
                    .package_digests
                    .get_mut("agentic-engineering")
                    .expect("base package") =
                    "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                        .to_owned();
            }
            _ => unreachable!(),
        }
        remint_selection_id(&mut wrong_candidate_binding);
        assert!(matches!(
            EngineeringAdvisoryAdoption::from_selection(
                &wrong_candidate_binding,
                &binding(&wrong_candidate_binding),
                AdvisoryAdoptionDisposition::Reuse,
                BTreeSet::from(["existing owner".to_owned()]),
                BTreeSet::new(),
                BTreeSet::from(["root check".to_owned()]),
            ),
            Err(AdvisoryError::StaleBinding)
        ));
    }
}
