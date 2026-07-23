use super::activation_inputs::{make_profile, prepare_skill_family};
use crate::context::LiveContext;
use crate::digest;
use crate::inventory::AuthorityCatalog;
use crate::plugin_product::skill_catalog::{
    CatalogWarning, HARNESS_FRONT_DOOR, HARNESS_PLUGIN, SkillCatalogRequest, legacy_aliases,
    project,
};
use serde::Serialize;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct SkillFamilyActivationProjection {
    candidate_id: String,
    catalog_id: String,
    source_snapshot_id: String,
    package_sha256: String,
    plugin_version: String,
    canonical_skill_names: Vec<String>,
    excluded_legacy_names: Vec<String>,
    profile_rejected_legacy: bool,
    implicit_gateways: Vec<String>,
    catalog_exclusion_sha256: String,
    profile_rejection_sha256: String,
    implicit_gateway_sha256: String,
    projection_sha256: String,
}

impl SkillFamilyActivationProjection {
    pub(crate) fn candidate_id(&self) -> &str {
        &self.candidate_id
    }

    pub(crate) fn catalog_id(&self) -> &str {
        &self.catalog_id
    }

    pub(crate) fn source_snapshot_id(&self) -> &str {
        &self.source_snapshot_id
    }

    pub(crate) fn package_sha256(&self) -> &str {
        &self.package_sha256
    }

    pub(crate) fn plugin_version(&self) -> &str {
        &self.plugin_version
    }

    pub(crate) fn projection_sha256(&self) -> &str {
        &self.projection_sha256
    }

    pub(crate) fn canonical_skill_names(&self) -> &[String] {
        &self.canonical_skill_names
    }

    pub(crate) fn excluded_legacy_names(&self) -> &[String] {
        &self.excluded_legacy_names
    }

    pub(crate) fn profile_rejected_legacy(&self) -> bool {
        self.profile_rejected_legacy
    }

    pub(crate) fn implicit_gateways(&self) -> &[String] {
        &self.implicit_gateways
    }

    pub(crate) fn catalog_exclusion_sha256(&self) -> &str {
        &self.catalog_exclusion_sha256
    }

    pub(crate) fn profile_rejection_sha256(&self) -> &str {
        &self.profile_rejection_sha256
    }

    pub(crate) fn implicit_gateway_sha256(&self) -> &str {
        &self.implicit_gateway_sha256
    }

    pub(crate) fn validate(&self) -> bool {
        super::activation_inputs::valid_digest(&self.candidate_id)
            && super::activation_inputs::valid_digest(&self.catalog_id)
            && super::activation_inputs::valid_digest(&self.source_snapshot_id)
            && super::activation_inputs::valid_digest(&self.package_sha256)
            && super::activation_inputs::valid_digest(&self.catalog_exclusion_sha256)
            && super::activation_inputs::valid_digest(&self.profile_rejection_sha256)
            && super::activation_inputs::valid_digest(&self.implicit_gateway_sha256)
            && super::activation_inputs::valid_digest(&self.projection_sha256)
            && !self.plugin_version.is_empty()
            && self.projection_sha256 == self.computed_projection_sha256()
    }

    fn computed_projection_sha256(&self) -> String {
        super::activation_inputs::projection_digest(self)
    }
}

pub(crate) fn observe_skill_family_activation(
    context: &LiveContext,
    catalog: &AuthorityCatalog,
) -> Result<SkillFamilyActivationProjection, crate::migration::product::ProductMigrationError> {
    let prepared = prepare_skill_family(context, catalog)?;
    let accepted_profile = make_profile(&prepared, prepared.canonical.clone());
    let projection = project(&SkillCatalogRequest {
        candidate_id: prepared.candidate_id.clone(),
        config_digest: prepared.catalog_id.clone(),
        profile: Some(accepted_profile),
        packages: vec![prepared.package.clone()],
    })
    .map_err(|_| {
        crate::migration::product::ProductMigrationError::new(
            "migration-product-skill-family-catalog-invalid",
        )
    })?;
    let profile_rejected_legacy = prepared.aliases.iter().all(|alias| {
        let rejected_profile = make_profile(&prepared, vec![alias.clone()]);
        project(&SkillCatalogRequest {
            candidate_id: prepared.candidate_id.clone(),
            config_digest: prepared.catalog_id.clone(),
            profile: Some(rejected_profile),
            packages: vec![prepared.package.clone()],
        })
        .is_err()
    });
    let aliases = prepared.aliases.clone();
    let excluded_legacy_names = aliases
        .into_iter()
        .filter(|name| {
            !prepared
                .packaged_paths
                .contains(&format!("skills/{name}/SKILL.md"))
        })
        .collect::<Vec<_>>();
    let implicit_gateways = projection.implicit_gateways.clone();
    let front_door_is_implicit = projection
        .skills
        .iter()
        .find(|skill| skill.name == HARNESS_FRONT_DOOR)
        .is_some_and(|skill| skill.allow_implicit_invocation);
    let warning_names = projection
        .warnings
        .iter()
        .filter_map(|warning| match warning {
            CatalogWarning::LegacyAliasExcluded { skill, .. } => Some(skill.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    let mut projected_names = projection
        .skills
        .iter()
        .map(|skill| skill.name.clone())
        .collect::<Vec<_>>();
    let mut expected_names = prepared.canonical.clone();
    projected_names.sort();
    expected_names.sort();
    if projected_names != expected_names
        || excluded_legacy_names
            != legacy_aliases()
                .iter()
                .map(|name| (*name).to_owned())
                .collect::<Vec<_>>()
        || !profile_rejected_legacy
        || warning_names
            != legacy_aliases()
                .iter()
                .map(|name| (*name).to_owned())
                .collect::<Vec<_>>()
        || implicit_gateways != [HARNESS_PLUGIN.to_owned()]
        || !front_door_is_implicit
    {
        return Err(crate::migration::product::ProductMigrationError::new(
            "migration-product-skill-family-authority-invalid",
        ));
    }
    let catalog_exclusion_sha256 = digest::bytes(
        &serde_json::to_vec(&(
            "n14-skill-family-catalog-exclusion-v3",
            &prepared.candidate_id,
            &prepared.catalog_id,
            &prepared.package_sha256,
            &prepared.canonical,
            &excluded_legacy_names,
            &warning_names,
        ))
        .map_err(|_| {
            crate::migration::product::ProductMigrationError::new(
                "migration-product-skill-family-catalog-digest-invalid",
            )
        })?,
    );
    let profile_rejection_sha256 = digest::bytes(
        &serde_json::to_vec(&(
            "n14-skill-family-profile-rejection-v3",
            &prepared.candidate_id,
            &prepared.catalog_id,
            &prepared.package_sha256,
            &prepared.aliases,
            profile_rejected_legacy,
        ))
        .map_err(|_| {
            crate::migration::product::ProductMigrationError::new(
                "migration-product-skill-family-profile-digest-invalid",
            )
        })?,
    );
    let implicit_gateway_sha256 = digest::bytes(
        &serde_json::to_vec(&(
            "n14-skill-family-implicit-gateway-v3",
            &prepared.candidate_id,
            &prepared.catalog_id,
            &prepared.package_sha256,
            &implicit_gateways,
            front_door_is_implicit,
        ))
        .map_err(|_| {
            crate::migration::product::ProductMigrationError::new(
                "migration-product-skill-family-gateway-digest-invalid",
            )
        })?,
    );
    let mut value = SkillFamilyActivationProjection {
        candidate_id: prepared.candidate_id,
        catalog_id: prepared.catalog_id,
        source_snapshot_id: prepared.source_snapshot_id,
        package_sha256: prepared.package_sha256,
        plugin_version: prepared.plugin_version,
        canonical_skill_names: prepared.canonical,
        excluded_legacy_names,
        profile_rejected_legacy,
        implicit_gateways,
        catalog_exclusion_sha256,
        profile_rejection_sha256,
        implicit_gateway_sha256,
        projection_sha256: String::new(),
    };
    value.projection_sha256 = value.computed_projection_sha256();
    Ok(value)
}
