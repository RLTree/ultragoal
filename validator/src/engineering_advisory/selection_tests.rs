use super::*;
use crate::plugin_product::skill_catalog::{
    AGENTIC_FILE_MANIFEST_DIGEST, AGENTIC_PLUGIN, AGENTIC_PLUGIN_VERSION, CatalogEffect,
    HARNESS_PLUGIN, PluginIdentity, ProfileIdentity, RenderedSkill, SkillCatalogProjection,
    coinstall_profile,
};
use std::collections::BTreeSet;

const CANDIDATE: &str = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const CONTEXT: &str = "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const STATE: &str = "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const CONFIG: &str = "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
const SELECTOR: &str = "EngineeringAdvisorySelector-v1";

fn request(
    stage: crate::plugin_product::skill_catalog::AgenticAdvisoryStage,
) -> AdvisorySelectionRequest {
    let profile = coinstall_profile(stage, CANDIDATE, CONFIG);
    let skills = profile
        .profile
        .selected_skills
        .iter()
        .map(|name| RenderedSkill {
            plugin: AGENTIC_PLUGIN.to_owned(),
            version: AGENTIC_PLUGIN_VERSION.to_owned(),
            name: name.clone(),
            path: format!("skills/{name}/agents/openai.yaml"),
            display_name: name.clone(),
            short_description: "bounded".to_owned(),
            default_prompt: "advise".to_owned(),
            allow_implicit_invocation: false,
            source_digest: CANDIDATE.to_owned(),
            rendered_characters: 1,
        })
        .collect::<Vec<_>>();
    AdvisorySelectionRequest {
        candidate_id: CANDIDATE.to_owned(),
        context_id: CONTEXT.to_owned(),
        product_state_id: STATE.to_owned(),
        config_digest: CONFIG.to_owned(),
        user_outcome_class: AdvisoryOutcomeClass::Routine,
        explicit_lens_requests: BTreeSet::new(),
        lifecycle_stage: "routine".to_owned(),
        active_truth_loop_id: Some("loop-1".to_owned()),
        first_broken_transition_id: Some("transition-1".to_owned()),
        protected_invariant_ids: BTreeSet::new(),
        contemplated_effects: BTreeSet::new(),
        authority_gaps: BTreeSet::new(),
        material_uncertainties: BTreeSet::new(),
        verification_oracle: None,
        false_pass_risks: BTreeSet::new(),
        failure_classes: BTreeSet::new(),
        recovery_ambiguities: BTreeSet::new(),
        product_fitness_gaps: BTreeSet::new(),
        activation_signals: BTreeSet::new(),
        plugin_version: AGENTIC_PLUGIN_VERSION.to_owned(),
        plugin_digest: AGENTIC_FILE_MANIFEST_DIGEST.to_owned(),
        profile_digest: profile.profile.digest.clone(),
        selector_version: SELECTOR.to_owned(),
        profile: Some(profile.clone()),
        catalog: Some(SkillCatalogProjection {
            schema_version: "PluginSkillCatalogProjection-v1".to_owned(),
            candidate_id: CANDIDATE.to_owned(),
            config_digest: CONFIG.to_owned(),
            profile: Some(ProfileIdentity {
                plugin: AGENTIC_PLUGIN.to_owned(),
                name: profile.profile.name.clone(),
                digest: profile.profile.digest.clone(),
                candidate_id: CANDIDATE.to_owned(),
                config_digest: CONFIG.to_owned(),
                plugin_version: AGENTIC_PLUGIN_VERSION.to_owned(),
                plugin_digest: AGENTIC_FILE_MANIFEST_DIGEST.to_owned(),
            }),
            plugins: vec![PluginIdentity {
                plugin: AGENTIC_PLUGIN.to_owned(),
                version: AGENTIC_PLUGIN_VERSION.to_owned(),
                plugin_digest: AGENTIC_FILE_MANIFEST_DIGEST.to_owned(),
                package_digest: STATE.to_owned(),
            }],
            enabled_skill_count: skills.len(),
            skills,
            rendered_characters: 100,
            character_limit: 8_000,
            headroom: 7_900,
            implicit_gateways: vec![HARNESS_PLUGIN.to_owned()],
            warnings: Vec::new(),
            effect: CatalogEffect::Read,
            claim_effect: false,
            projection_digest: CONTEXT.to_owned(),
        }),
        prior_selection: None,
    }
}

#[test]
fn every_agentic_lens_has_a_positive_stage_profile_route() {
    let stages = [
        crate::plugin_product::skill_catalog::AgenticAdvisoryStage::Core,
        crate::plugin_product::skill_catalog::AgenticAdvisoryStage::Lifecycle,
        crate::plugin_product::skill_catalog::AgenticAdvisoryStage::Rust,
    ];
    for lens in AdvisoryLens::all() {
        let stage = stages
            .iter()
            .copied()
            .find(|stage| {
                coinstall_profile(*stage, CANDIDATE, CONFIG)
                    .profile
                    .selected_skills
                    .iter()
                    .any(|skill| skill == lens.skill_name())
            })
            .expect("every lens is covered by a stage profile");
        let mut current = request(stage);
        current.activation_signals.insert(*lens);
        let selected = select_advisory(&current);
        assert_eq!(
            selected.disposition,
            AdvisorySelectionDisposition::AdvisorySelected
        );
        assert_eq!(selected.primary_lens, Some(*lens));
        assert_eq!(selected.claim_ceiling, "proposal_only_no_claim");
    }
}

#[test]
fn no_change_and_unchanged_advice_do_not_create_an_activation_loop() {
    let current = request(crate::plugin_product::skill_catalog::AgenticAdvisoryStage::Core);
    assert_eq!(
        select_advisory(&current).disposition,
        AdvisorySelectionDisposition::NoAdvisoryNeeded
    );

    let mut selected_request = current;
    selected_request
        .activation_signals
        .insert(AdvisoryLens::CodexTaskContract);
    let selected = select_advisory(&selected_request);
    selected_request.prior_selection = Some(Box::new(selected));
    let repeated = select_advisory(&selected_request);
    assert_eq!(
        repeated.disposition,
        AdvisorySelectionDisposition::NoAdvisoryNeeded
    );
    assert!(repeated.activation_reasons[0].starts_with("advice_current"));
}

#[test]
fn partial_change_and_profile_substitution_fail_in_the_right_direction() {
    let mut current = request(crate::plugin_product::skill_catalog::AgenticAdvisoryStage::Core);
    current.user_outcome_class = AdvisoryOutcomeClass::MaterialDecision;
    assert_eq!(
        select_advisory(&current).primary_lens,
        Some(AdvisoryLens::CodexTaskContract)
    );

    current
        .catalog
        .as_mut()
        .unwrap()
        .implicit_gateways
        .push(AGENTIC_PLUGIN.to_owned());
    assert_eq!(
        select_advisory(&current).disposition,
        AdvisorySelectionDisposition::RequiredProfileUnavailable
    );

    current.catalog.as_mut().unwrap().implicit_gateways = vec![HARNESS_PLUGIN.to_owned()];
    current.catalog.as_mut().unwrap().candidate_id = CONTEXT.to_owned();
    assert_eq!(
        select_advisory(&current).disposition,
        AdvisorySelectionDisposition::StaleOrCrossCandidate
    );
}

#[test]
fn cross_layer_selection_refuses_to_drop_a_required_supporting_lens() {
    let mut current = request(crate::plugin_product::skill_catalog::AgenticAdvisoryStage::Core);
    current
        .activation_signals
        .insert(AdvisoryLens::CodexTaskContract);
    current
        .activation_signals
        .insert(AdvisoryLens::ProductFitnessEngineering);

    assert_eq!(
        select_advisory(&current).disposition,
        AdvisorySelectionDisposition::RequiredProfileUnavailable
    );
}

#[test]
fn selection_rejects_a_profile_with_an_omitted_rendered_skill() {
    let mut current = request(crate::plugin_product::skill_catalog::AgenticAdvisoryStage::Core);
    current
        .activation_signals
        .insert(AdvisoryLens::CodexTaskContract);
    current.catalog.as_mut().expect("catalog").skills.pop();
    assert_eq!(
        select_advisory(&current).disposition,
        AdvisorySelectionDisposition::RequiredProfileUnavailable
    );
}
