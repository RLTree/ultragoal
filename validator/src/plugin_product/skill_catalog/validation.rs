use super::model::*;
use super::projection::is_legacy_alias_for_plugin;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn validate_package(
    request: &SkillCatalogRequest,
    package: &SkillPackage,
) -> Result<(), SkillCatalogError> {
    if package.plugin.trim().is_empty() {
        return Err(SkillCatalogError::PluginVersionMismatch(
            "empty plugin".to_owned(),
        ));
    }
    validate_digest(&package.plugin_digest, "plugin_digest")?;
    validate_digest(&package.package_digest, "package_digest")?;
    if package.candidate_id != request.candidate_id {
        return Err(SkillCatalogError::CrossCandidate(package.plugin.clone()));
    }
    if package.config_digest != request.config_digest {
        return Err(SkillCatalogError::ConfigMismatch(package.plugin.clone()));
    }
    if package.version.trim().is_empty() {
        return Err(SkillCatalogError::PluginVersionMismatch(
            package.plugin.clone(),
        ));
    }
    for skill in &package.skills {
        if skill.path.trim().is_empty() || skill.path.starts_with('/') || skill.path.contains("..")
        {
            return Err(SkillCatalogError::InvalidPath(skill.path.clone()));
        }
    }
    Ok(())
}

pub(super) fn validate_profile(
    request: &SkillCatalogRequest,
) -> Result<Option<SkillProfile>, SkillCatalogError> {
    let Some(profile) = request.profile.clone() else {
        return Ok(None);
    };
    validate_digest(&profile.digest, "profile_digest")?;
    validate_digest(&profile.plugin_digest, "profile_plugin_digest")?;
    if profile.candidate_id != request.candidate_id {
        return Err(SkillCatalogError::CrossCandidate("profile".to_owned()));
    }
    if profile.config_digest != request.config_digest {
        return Err(SkillCatalogError::StaleProfile("config_digest"));
    }
    if profile.plugin.trim().is_empty() || profile.name.trim().is_empty() {
        return Err(SkillCatalogError::StaleProfile("identity"));
    }
    let mut names = BTreeSet::new();
    for skill in &profile.selected_skills {
        if !names.insert(skill) {
            return Err(SkillCatalogError::DuplicateProfileSkill(skill.clone()));
        }
    }
    Ok(Some(profile))
}

pub(super) fn validate_profile_selection(
    packages: &BTreeMap<String, &SkillPackage>,
    rendered: &[RenderedSkill],
    profile: Option<&SkillProfile>,
) -> Result<(), SkillCatalogError> {
    let Some(profile) = profile else {
        return Ok(());
    };
    let package = packages
        .get(&profile.plugin)
        .ok_or_else(|| SkillCatalogError::ProfilePluginMissing(profile.plugin.clone()))?;
    if package.version != profile.plugin_version || package.plugin_digest != profile.plugin_digest {
        return Err(SkillCatalogError::StaleProfile("plugin_version_or_digest"));
    }
    let package_names = package
        .skills
        .iter()
        .filter(|skill| skill.enabled && !is_legacy_alias_for_plugin(&package.plugin, &skill.name))
        .map(|skill| skill.name.as_str())
        .collect::<BTreeSet<_>>();
    let selected = profile
        .selected_skills
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if let Some(unknown) = selected.difference(&package_names).next() {
        if package.skills.iter().any(|skill| skill.name == *unknown) {
            return Err(SkillCatalogError::ExtraProfileSkill((*unknown).to_owned()));
        }
        return Err(SkillCatalogError::UnknownProfileSkill(
            (*unknown).to_owned(),
        ));
    }
    if let Some(omitted) = package_names.difference(&selected).next() {
        return Err(SkillCatalogError::OmittedProfileSkill(
            (*omitted).to_owned(),
        ));
    }
    if rendered
        .iter()
        .filter(|skill| skill.plugin == profile.plugin)
        .map(|skill| skill.name.as_str())
        .collect::<BTreeSet<_>>()
        != selected
    {
        return Err(SkillCatalogError::OmittedProfileSkill(
            profile.plugin.clone(),
        ));
    }
    Ok(())
}

pub(super) fn validate_digest(value: &str, field: &'static str) -> Result<(), SkillCatalogError> {
    if value.len() != 71
        || !value.starts_with("sha256:")
        || !value[7..].bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(SkillCatalogError::InvalidDigest(field));
    }
    Ok(())
}
