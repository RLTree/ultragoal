use super::{
    ADVISORY_SELECTION_REQUEST_SCHEMA, ADVISORY_SELECTOR_VERSION, AdvisorySelectionRequest,
};
use crate::plugin_product::engineering_advisory::{exact_agentic_candidate_binding, qualify_skill};
use crate::plugin_product::skill_catalog::{CatalogEffect, HARNESS_PLUGIN, coinstall_profile};
use std::collections::BTreeSet;

pub(super) fn identity_is_well_formed(request: &AdvisorySelectionRequest) -> bool {
    [
        &request.candidate_id,
        &request.context_id,
        &request.product_state_id,
        &request.config_digest,
        &request.pack_set_digest,
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
        profile.candidate_id != request.candidate_id
            || profile.config_digest != request.config_digest
    }) || request.catalog.as_ref().is_some_and(|catalog| {
        catalog.candidate_id != request.candidate_id
            || catalog.config_digest != request.config_digest
    })
}

pub(super) fn available_skill_names(
    request: &AdvisorySelectionRequest,
) -> Option<BTreeSet<String>> {
    let profile = request.profile.as_ref()?;
    let catalog = request.catalog.as_ref()?;
    let expected = coinstall_profile(
        profile.stage,
        request.candidate_id.clone(),
        request.config_digest.clone(),
    );
    let one_harness_gateway = catalog.implicit_gateways.len() == 1
        && catalog.implicit_gateways.first().map(String::as_str) == Some(HARNESS_PLUGIN);
    let expected_pack_names = profile
        .pack_set
        .packs
        .iter()
        .map(|pack| pack.name.as_str())
        .collect::<BTreeSet<_>>();
    let rendered_agentic_skills = catalog
        .skills
        .iter()
        .filter(|skill| {
            expected_pack_names.contains(skill.plugin.as_str()) && !skill.allow_implicit_invocation
        })
        .map(|skill| format!("{}:{}", skill.plugin, skill.name))
        .collect::<BTreeSet<_>>();
    let rendered_agentic_skill_count = catalog
        .skills
        .iter()
        .filter(|skill| expected_pack_names.contains(skill.plugin.as_str()))
        .count();
    let expected_agentic_skills = profile
        .selected_skills
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let exact_binding = exact_agentic_candidate_binding();
    let observed_pack_identities = catalog
        .plugins
        .iter()
        .filter(|plugin| expected_pack_names.contains(plugin.plugin.as_str()))
        .map(|plugin| (plugin.plugin.as_str(), plugin))
        .collect::<std::collections::BTreeMap<_, _>>();
    let observed_pack_identity_count = catalog
        .plugins
        .iter()
        .filter(|plugin| expected_pack_names.contains(plugin.plugin.as_str()))
        .count();
    let unexpected_agentic_identity = catalog.plugins.iter().any(|plugin| {
        plugin.plugin.starts_with("agentic-engineering")
            && !expected_pack_names.contains(plugin.plugin.as_str())
    });
    let rendered_identity_mismatch = catalog.skills.iter().any(|skill| {
        if !expected_pack_names.contains(skill.plugin.as_str()) {
            return skill.plugin.starts_with("agentic-engineering");
        }
        profile
            .pack_set
            .packs
            .iter()
            .find(|pack| pack.name == skill.plugin)
            .is_none_or(|pack| skill.version != pack.version)
    });
    if profile != &expected
        || profile.pack_set.validate_against(&exact_binding).is_err()
        || request.pack_set_digest != profile.pack_set.aggregate_digest
        || request.profile_digest != profile.source_profile_digest
        || request.schema_version != ADVISORY_SELECTION_REQUEST_SCHEMA
        || request.selector_version != ADVISORY_SELECTOR_VERSION
        || catalog.candidate_id != request.candidate_id
        || catalog.config_digest != request.config_digest
        || catalog.profile.is_some()
        || catalog.effect != CatalogEffect::Read
        || catalog.claim_effect
        || !one_harness_gateway
        || unexpected_agentic_identity
        || rendered_identity_mismatch
        || observed_pack_identity_count != profile.pack_set.packs.len()
        || observed_pack_identities.len() != profile.pack_set.packs.len()
        || rendered_agentic_skill_count != expected_agentic_skills.len()
        || profile.pack_set.packs.iter().any(|pack| {
            observed_pack_identities
                .get(pack.name.as_str())
                .is_none_or(|plugin| {
                    plugin.version != pack.version
                        || plugin.plugin_digest != pack.manifest_digest
                        || profile.candidate_binding.package_digests.get(&pack.name)
                            != Some(&plugin.package_digest)
                })
        })
        || rendered_agentic_skills != expected_agentic_skills
    {
        return None;
    }
    Some(rendered_agentic_skills)
}

pub(super) fn qualified_skill_name(skill: &str) -> Option<String> {
    qualify_skill(skill)
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
    if request.selector_version != ADVISORY_SELECTOR_VERSION {
        missing.insert("selector_version".to_owned());
    }
    if request.schema_version != ADVISORY_SELECTION_REQUEST_SCHEMA {
        missing.insert("request_schema_version".to_owned());
    }
    missing
}
