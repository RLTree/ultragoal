use super::AdvisorySelectionRequest;
use crate::plugin_product::skill_catalog::{
    AGENTIC_PLUGIN, AGENTIC_PLUGIN_VERSION, HARNESS_PLUGIN, coinstall_profile,
};
use std::collections::BTreeSet;

pub(super) const SELECTOR_VERSION: &str = "EngineeringAdvisorySelector-v1";

pub(super) fn identity_is_well_formed(request: &AdvisorySelectionRequest) -> bool {
    [
        &request.candidate_id,
        &request.context_id,
        &request.product_state_id,
        &request.config_digest,
        &request.plugin_digest,
        &request.profile_digest,
    ]
    .into_iter()
    .all(|value| {
        value.len() == 71
            && value.starts_with("sha256:")
            && value[7..].bytes().all(|byte| byte.is_ascii_hexdigit())
    })
}

pub(super) fn binding_is_stale(request: &AdvisorySelectionRequest) -> bool {
    request.profile.as_ref().is_some_and(|profile| {
        profile.profile.candidate_id != request.candidate_id
            || profile.profile.config_digest != request.config_digest
    }) || request.catalog.as_ref().is_some_and(|catalog| {
        catalog.candidate_id != request.candidate_id
            || catalog.config_digest != request.config_digest
    })
}

pub(super) fn available_skill_names(request: &AdvisorySelectionRequest) -> Option<BTreeSet<&str>> {
    let profile = request.profile.as_ref()?;
    let catalog = request.catalog.as_ref()?;
    let expected = coinstall_profile(
        profile.stage,
        request.candidate_id.clone(),
        request.config_digest.clone(),
    );
    let one_harness_gateway = catalog.implicit_gateways.len() == 1
        && catalog.implicit_gateways.first().map(String::as_str) == Some(HARNESS_PLUGIN);
    let rendered_agentic_skills = catalog
        .skills
        .iter()
        .filter(|skill| skill.plugin == AGENTIC_PLUGIN && !skill.allow_implicit_invocation)
        .map(|skill| skill.name.as_str())
        .collect::<BTreeSet<_>>();
    let expected_agentic_skills = profile
        .profile
        .selected_skills
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if profile != &expected
        || request.plugin_version != AGENTIC_PLUGIN_VERSION
        || request.plugin_digest != profile.profile.plugin_digest
        || request.profile_digest != profile.profile.digest
        || request.selector_version != SELECTOR_VERSION
        || catalog.candidate_id != request.candidate_id
        || catalog.config_digest != request.config_digest
        || catalog
            .profile
            .as_ref()
            .map(|identity| identity.digest.as_str())
            != Some(profile.profile.digest.as_str())
        || !one_harness_gateway
        || !catalog.plugins.iter().any(|plugin| {
            plugin.plugin == AGENTIC_PLUGIN
                && plugin.version == request.plugin_version
                && plugin.plugin_digest == request.plugin_digest
        })
        || rendered_agentic_skills != expected_agentic_skills
    {
        return None;
    }
    Some(rendered_agentic_skills)
}

pub(super) fn missing_inputs(request: &AdvisorySelectionRequest) -> BTreeSet<String> {
    let mut missing = BTreeSet::new();
    for (field, value) in [
        ("candidate_id", request.candidate_id.as_str()),
        ("context_id", request.context_id.as_str()),
        ("product_state_id", request.product_state_id.as_str()),
        ("config_digest", request.config_digest.as_str()),
        ("lifecycle_stage", request.lifecycle_stage.as_str()),
    ] {
        if value.trim().is_empty() {
            missing.insert(field.to_owned());
        }
    }
    if request.selector_version != SELECTOR_VERSION {
        missing.insert("selector_version".to_owned());
    }
    missing
}
