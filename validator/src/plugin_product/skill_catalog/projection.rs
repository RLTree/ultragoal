use super::model::*;
use super::parser::{ParsedSkillMetadata, parse_skill_metadata};
use super::validation::{
    validate_digest, validate_package, validate_profile, validate_profile_selection,
};
use crate::digest;
use std::collections::{BTreeMap, BTreeSet};

const LEGACY_ALIASES: &[&str] = &[
    "agent-first-repo-init",
    "agent-first-repo-retrofit",
    "agent-improvement-loop",
    "agent-observability-stack",
    "agent-runtime-legibility",
    "execplan-lane",
    "fit-repo",
    "harness-engineering",
    "orchestrator-reconciler",
    "product-cohesion-gate",
    "product-fitness-gate",
    "proof-gate",
    "standards-gardener",
    "ultragoal",
];

pub fn project(request: &SkillCatalogRequest) -> Result<SkillCatalogProjection, SkillCatalogError> {
    validate_digest(&request.candidate_id, "candidate_id")?;
    validate_digest(&request.config_digest, "config_digest")?;
    if request.packages.is_empty() {
        return Err(SkillCatalogError::EmptyPackages);
    }
    let profile = validate_profile(request)?;
    let packages = selected_packages(request)?;
    let (mut skills, warnings, gateways) = render_selected_skills(&packages, profile.as_ref())?;
    validate_profile_selection(&packages, &skills, profile.as_ref())?;
    if let Some(gateway) = gateways
        .iter()
        .find(|gateway| gateway.as_str() != HARNESS_PLUGIN)
    {
        if gateways.len() == 1 {
            return Err(SkillCatalogError::UnexpectedImplicitSkill(gateway.clone()));
        }
    }
    if gateways.is_empty() {
        return Err(SkillCatalogError::MissingImplicitGateway);
    }
    if gateways.len() > 1 {
        return Err(SkillCatalogError::MultipleImplicitGateways(
            gateways.into_iter().collect(),
        ));
    }
    skills.sort_by(|left, right| (&left.plugin, &left.name).cmp(&(&right.plugin, &right.name)));
    let rendered_characters = skills
        .iter()
        .try_fold(0usize, |total, skill| {
            total.checked_add(skill.rendered_characters)
        })
        .ok_or(SkillCatalogError::OverBudget {
            estimate: usize::MAX,
            limit: DISCOVERY_CHARACTER_LIMIT,
        })?;
    if rendered_characters > DISCOVERY_CHARACTER_LIMIT {
        return Err(SkillCatalogError::OverBudget {
            estimate: rendered_characters,
            limit: DISCOVERY_CHARACTER_LIMIT,
        });
    }
    let mut plugins = packages
        .values()
        .map(|package| PluginIdentity {
            plugin: package.plugin.clone(),
            version: package.version.clone(),
            plugin_digest: package.plugin_digest.clone(),
            package_digest: package.package_digest.clone(),
        })
        .collect::<Vec<_>>();
    plugins.sort_by(|left, right| left.plugin.cmp(&right.plugin));
    let mut projection = SkillCatalogProjection {
        schema_version: "PluginSkillCatalogProjection-v1".to_owned(),
        candidate_id: request.candidate_id.clone(),
        config_digest: request.config_digest.clone(),
        profile: profile.map(profile_identity),
        plugins,
        enabled_skill_count: skills.len(),
        skills,
        rendered_characters,
        character_limit: DISCOVERY_CHARACTER_LIMIT,
        headroom: DISCOVERY_CHARACTER_LIMIT - rendered_characters,
        implicit_gateways: gateways.into_iter().collect(),
        warnings,
        effect: CatalogEffect::Read,
        claim_effect: false,
        projection_digest: String::new(),
    };
    projection.projection_digest = digest::bytes(
        &serde_json::to_vec(&projection)
            .map_err(|_| SkillCatalogError::InvalidDigest("projection"))?,
    );
    Ok(projection)
}

fn selected_packages(
    request: &SkillCatalogRequest,
) -> Result<BTreeMap<String, &SkillPackage>, SkillCatalogError> {
    let mut packages = BTreeMap::new();
    for package in request.packages.iter().filter(|package| package.selected) {
        validate_package(request, package)?;
        if packages.insert(package.plugin.clone(), package).is_some() {
            return Err(SkillCatalogError::DuplicatePackage(package.plugin.clone()));
        }
    }
    if packages.is_empty() {
        return Err(SkillCatalogError::EmptyPackages);
    }
    Ok(packages)
}

fn render_selected_skills(
    packages: &BTreeMap<String, &SkillPackage>,
    profile: Option<&SkillProfile>,
) -> Result<(Vec<RenderedSkill>, Vec<CatalogWarning>, BTreeSet<String>), SkillCatalogError> {
    let mut rendered = Vec::new();
    let mut warnings = Vec::new();
    let mut names = BTreeSet::new();
    let mut gateways = BTreeSet::new();
    for package in packages.values() {
        let selected = profile
            .filter(|profile| profile.plugin == package.plugin)
            .map(|profile| profile.selected_skills.iter().collect::<BTreeSet<_>>());
        for skill in &package.skills {
            if is_legacy_alias_for_plugin(&package.plugin, &skill.name) {
                warnings.push(CatalogWarning::LegacyAliasExcluded {
                    plugin: package.plugin.clone(),
                    skill: skill.name.clone(),
                });
                if selected
                    .as_ref()
                    .is_some_and(|names| names.contains(&skill.name))
                {
                    return Err(SkillCatalogError::LegacyAliasSelected(skill.name.clone()));
                }
                continue;
            }
            if !skill.enabled {
                warnings.push(CatalogWarning::OmittedSkill {
                    plugin: package.plugin.clone(),
                    skill: skill.name.clone(),
                });
                continue;
            }
            if selected
                .as_ref()
                .is_some_and(|names| !names.contains(&skill.name))
            {
                continue;
            }
            if !names.insert(skill.name.clone()) {
                return Err(SkillCatalogError::DuplicateSkill(skill.name.clone()));
            }
            let metadata = parse_skill_metadata(&skill.openai_yaml).map_err(|error| {
                SkillCatalogError::ParseSkill(skill.name.clone(), format!("{error:?}"))
            })?;
            if metadata.allow_implicit_invocation {
                if package.plugin == HARNESS_PLUGIN && skill.name != HARNESS_FRONT_DOOR {
                    return Err(SkillCatalogError::UnexpectedImplicitSkill(
                        skill.name.clone(),
                    ));
                }
                gateways.insert(package.plugin.clone());
            } else if package.plugin == HARNESS_PLUGIN && skill.name == HARNESS_FRONT_DOOR {
                return Err(SkillCatalogError::InvalidImplicitPolicy(skill.name.clone()));
            }
            if metadata.short_description.trim_end().ends_with('…')
                || metadata.short_description.trim_end().ends_with("...")
            {
                warnings.push(CatalogWarning::TruncatedDescription {
                    plugin: package.plugin.clone(),
                    skill: skill.name.clone(),
                });
            }
            rendered.push(render_skill(package, skill, metadata));
        }
    }
    Ok((rendered, warnings, gateways))
}

fn render_skill(
    package: &SkillPackage,
    skill: &SkillArtifact,
    metadata: ParsedSkillMetadata,
) -> RenderedSkill {
    let rendered_characters = package.plugin.len()
        + 1
        + skill.name.len()
        + 1
        + metadata.display_name.len()
        + 1
        + metadata.short_description.len()
        + 1
        + metadata.default_prompt.len()
        + 1;
    RenderedSkill {
        plugin: package.plugin.clone(),
        version: package.version.clone(),
        name: skill.name.clone(),
        path: skill.path.clone(),
        display_name: metadata.display_name,
        short_description: metadata.short_description,
        default_prompt: metadata.default_prompt,
        allow_implicit_invocation: metadata.allow_implicit_invocation,
        source_digest: digest::bytes(skill.openai_yaml.as_bytes()),
        rendered_characters,
    }
}

fn profile_identity(profile: SkillProfile) -> ProfileIdentity {
    ProfileIdentity {
        plugin: profile.plugin,
        name: profile.name,
        digest: profile.digest,
        candidate_id: profile.candidate_id,
        config_digest: profile.config_digest,
        plugin_version: profile.plugin_version,
        plugin_digest: profile.plugin_digest,
    }
}

pub(crate) fn is_legacy_alias_for_plugin(plugin: &str, name: &str) -> bool {
    plugin == HARNESS_PLUGIN && LEGACY_ALIASES.contains(&name)
}
