use super::*;
use crate::plugin_product::engineering_advisory::{
    AGENTIC_PACK_SET_DIGEST, exact_agentic_candidate_binding, exact_agentic_pack_set, qualify_skill,
};
use crate::plugin_product::skill_catalog::{
    CatalogEffect, HARNESS_PLUGIN, PluginIdentity, RenderedSkill, SkillCatalogProjection,
    coinstall_profile,
};
use std::collections::BTreeSet;

const CANDIDATE: &str = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const CONTEXT: &str = "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const STATE: &str = "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const CONFIG: &str = "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
const REQUEST_SCHEMA: &str = "AdvisorySelectionRequest-v2";
const SELECTOR: &str = "EngineeringAdvisorySelector-v2";

fn request(
    stage: crate::plugin_product::skill_catalog::AgenticAdvisoryStage,
) -> AdvisorySelectionRequest {
    let profile = coinstall_profile(stage, CANDIDATE, CONFIG);
    let skills = profile
        .selected_skills
        .iter()
        .map(|qualified| {
            let (plugin, name) = qualified.split_once(':').expect("qualified skill");
            RenderedSkill {
                plugin: plugin.to_owned(),
                version: "4.0.0".to_owned(),
                name: name.to_owned(),
                path: format!("skills/{name}/agents/openai.yaml"),
                display_name: name.to_owned(),
                short_description: "bounded".to_owned(),
                default_prompt: "advise".to_owned(),
                allow_implicit_invocation: false,
                source_digest: CANDIDATE.to_owned(),
                rendered_characters: 1,
            }
        })
        .collect::<Vec<_>>();
    let candidate_binding = exact_agentic_candidate_binding();
    let plugins = exact_agentic_pack_set()
        .packs
        .into_iter()
        .map(|pack| PluginIdentity {
            package_digest: candidate_binding
                .package_digests
                .get(&pack.name)
                .expect("exact package digest")
                .clone(),
            plugin: pack.name,
            version: pack.version,
            plugin_digest: pack.manifest_digest,
        })
        .collect();
    AdvisorySelectionRequest {
        schema_version: REQUEST_SCHEMA.to_owned(),
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
        pack_set_digest: AGENTIC_PACK_SET_DIGEST.to_owned(),
        profile_digest: profile.source_profile_digest.clone(),
        selector_version: SELECTOR.to_owned(),
        profile: Some(profile),
        catalog: Some(SkillCatalogProjection {
            schema_version: "PluginSkillCatalogProjection-v1".to_owned(),
            candidate_id: CANDIDATE.to_owned(),
            config_digest: CONFIG.to_owned(),
            profile: None,
            plugins,
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
fn every_agentic_lens_has_a_fully_qualified_stage_route() {
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
                let qualified = qualify_skill(lens.skill_name()).expect("owned lens");
                coinstall_profile(*stage, CANDIDATE, CONFIG)
                    .selected_skills
                    .contains(&qualified)
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
        assert_eq!(
            selected.qualified_primary_skill,
            qualify_skill(lens.skill_name())
        );
        assert_eq!(selected.schema_version, "EngineeringAdvisorySelection-v2");
        assert_eq!(selected.pack_set_digest, AGENTIC_PACK_SET_DIGEST);
        assert_eq!(selected.selector_version, SELECTOR);
        assert_eq!(
            selected.agentic_candidate,
            Some(exact_agentic_candidate_binding())
        );
        assert_eq!(selected.claim_ceiling, "proposal_only_no_claim");
    }
}

#[test]
fn per_pack_identity_change_invalidates_the_selection_fingerprint() {
    let mut exact = request(crate::plugin_product::skill_catalog::AgenticAdvisoryStage::Core);
    exact
        .activation_signals
        .insert(AdvisoryLens::CodexTaskContract);
    let selected = select_advisory(&exact);
    assert_eq!(
        selected.disposition,
        AdvisorySelectionDisposition::AdvisorySelected
    );

    let mut changed = exact;
    changed.profile.as_mut().expect("profile").pack_set.packs[0].manifest_digest =
        "sha256:23bae48d6063fe876fc644e0f1b25b34964b591c149bfc7cf0e8b3e81fa4113a".to_owned();
    let refused = select_advisory(&changed);
    assert_eq!(
        refused.disposition,
        AdvisorySelectionDisposition::RequiredProfileUnavailable
    );
    assert_ne!(selected.input_fingerprint, refused.input_fingerprint);
}

#[test]
fn legacy_request_or_profile_wire_version_is_rejected() {
    let mut legacy_request =
        request(crate::plugin_product::skill_catalog::AgenticAdvisoryStage::Core);
    legacy_request
        .activation_signals
        .insert(AdvisoryLens::CodexTaskContract);
    legacy_request.schema_version = "AdvisorySelectionRequest-v1".to_owned();
    let missing = select_advisory(&legacy_request);
    assert_eq!(
        missing.disposition,
        AdvisorySelectionDisposition::MaterialInputMissing
    );
    assert!(missing.missing_inputs.contains("request_schema_version"));

    let mut legacy_profile =
        request(crate::plugin_product::skill_catalog::AgenticAdvisoryStage::Core);
    legacy_profile
        .activation_signals
        .insert(AdvisoryLens::CodexTaskContract);
    legacy_profile
        .profile
        .as_mut()
        .expect("profile")
        .schema_version = "AgenticCoInstallProfile-v1".to_owned();
    assert_eq!(
        select_advisory(&legacy_profile).disposition,
        AdvisorySelectionDisposition::RequiredProfileUnavailable
    );
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
fn profile_substitution_and_second_gateway_fail_closed() {
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
        .push("agentic-engineering".to_owned());
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
fn missing_companion_pack_returns_typed_unavailable() {
    let mut current = request(crate::plugin_product::skill_catalog::AgenticAdvisoryStage::Rust);
    current
        .activation_signals
        .insert(AdvisoryLens::RustAgentDurability);
    current
        .catalog
        .as_mut()
        .expect("catalog")
        .plugins
        .retain(|plugin| plugin.plugin != "agentic-engineering-systems");
    assert_eq!(
        select_advisory(&current).disposition,
        AdvisorySelectionDisposition::RequiredProfileUnavailable
    );
}

#[test]
fn duplicate_pack_identity_or_wrong_rendered_version_is_unavailable() {
    let mut duplicate = request(crate::plugin_product::skill_catalog::AgenticAdvisoryStage::Core);
    duplicate
        .activation_signals
        .insert(AdvisoryLens::CodexTaskContract);
    let repeated = duplicate
        .catalog
        .as_ref()
        .expect("catalog")
        .plugins
        .first()
        .expect("pack")
        .clone();
    duplicate
        .catalog
        .as_mut()
        .expect("catalog")
        .plugins
        .push(repeated);
    assert_eq!(
        select_advisory(&duplicate).disposition,
        AdvisorySelectionDisposition::RequiredProfileUnavailable
    );

    let mut wrong_version =
        request(crate::plugin_product::skill_catalog::AgenticAdvisoryStage::Core);
    wrong_version
        .activation_signals
        .insert(AdvisoryLens::CodexTaskContract);
    wrong_version
        .catalog
        .as_mut()
        .expect("catalog")
        .skills
        .first_mut()
        .expect("rendered skill")
        .version = "3.0.1".to_owned();
    assert_eq!(
        select_advisory(&wrong_version).disposition,
        AdvisorySelectionDisposition::RequiredProfileUnavailable
    );

    let mut wrong_package =
        request(crate::plugin_product::skill_catalog::AgenticAdvisoryStage::Core);
    wrong_package
        .activation_signals
        .insert(AdvisoryLens::CodexTaskContract);
    wrong_package
        .catalog
        .as_mut()
        .expect("catalog")
        .plugins
        .first_mut()
        .expect("pack")
        .package_digest =
        "sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".to_owned();
    assert_eq!(
        select_advisory(&wrong_package).disposition,
        AdvisorySelectionDisposition::RequiredProfileUnavailable
    );
}

#[test]
fn claim_bearing_catalog_is_unavailable() {
    let mut current = request(crate::plugin_product::skill_catalog::AgenticAdvisoryStage::Core);
    current
        .activation_signals
        .insert(AdvisoryLens::CodexTaskContract);
    current.catalog.as_mut().expect("catalog").claim_effect = true;
    assert_eq!(
        select_advisory(&current).disposition,
        AdvisorySelectionDisposition::RequiredProfileUnavailable
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
fn selection_rejects_an_omitted_or_unqualified_rendered_skill() {
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
