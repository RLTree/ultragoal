use crate::context::LiveContext;
use crate::digest;
use crate::distribution::{PackageEntry, PackageSnapshot, capture_product_package};
use crate::inventory::AuthorityCatalog;
use crate::migration::product::ProductMigrationError;
use crate::plugin_product::skill_catalog::{
    HARNESS_PLUGIN, SkillArtifact, SkillPackage, SkillProfile, legacy_aliases,
};
use std::collections::BTreeSet;

use super::activation::SkillFamilyActivationProjection;

pub(crate) struct PreparedSkillFamily {
    pub(crate) candidate_id: String,
    pub(crate) catalog_id: String,
    pub(crate) source_snapshot_id: String,
    pub(crate) package_sha256: String,
    pub(crate) plugin_version: String,
    pub(crate) canonical: Vec<String>,
    pub(crate) aliases: Vec<String>,
    pub(crate) package: SkillPackage,
    pub(crate) packaged_paths: BTreeSet<String>,
}

pub(crate) fn prepare_skill_family(
    context: &LiveContext,
    catalog: &AuthorityCatalog,
) -> Result<PreparedSkillFamily, ProductMigrationError> {
    let artifact = capture_product_package(context, catalog).map_err(|_| {
        ProductMigrationError::new("migration-product-skill-family-authority-unavailable")
    })?;
    let snapshot = artifact.snapshot();
    let canonical = crate::distribution::canonical_skill_names()
        .iter()
        .map(|name| (*name).to_owned())
        .collect::<Vec<_>>();
    let aliases = legacy_aliases()
        .iter()
        .map(|name| (*name).to_owned())
        .collect::<Vec<_>>();
    let mut compatibility_entries = canonical
        .iter()
        .map(|name| {
            let yaml_path = format!("skills/{name}/{}{}", "agents", "/openai.yaml");
            let yaml = entry(snapshot, &yaml_path)
                .and_then(|entry| std::str::from_utf8(entry.bytes()).ok())
                .ok_or_else(|| {
                    ProductMigrationError::new("migration-product-skill-family-yaml-unavailable")
                })?;
            Ok(SkillArtifact {
                name: name.clone(),
                path: yaml_path,
                openai_yaml: yaml.to_owned(),
                enabled: true,
            })
        })
        .collect::<Result<Vec<_>, ProductMigrationError>>()?;
    let plugin_entry = entry(snapshot, ".codex-plugin/plugin.json").ok_or_else(|| {
        ProductMigrationError::new("migration-product-skill-family-package-input-unavailable")
    })?;
    let plugin =
        serde_json::from_slice::<serde_json::Value>(plugin_entry.bytes()).map_err(|_| {
            ProductMigrationError::new("migration-product-skill-family-package-input-invalid")
        })?;
    let plugin_version = plugin
        .get("version")
        .and_then(serde_json::Value::as_str)
        .filter(|version| !version.is_empty())
        .ok_or_else(|| {
            ProductMigrationError::new("migration-product-skill-family-package-version-unavailable")
        })?
        .to_owned();
    if plugin.get("name").and_then(serde_json::Value::as_str) != Some(HARNESS_PLUGIN) {
        return Err(ProductMigrationError::new(
            "migration-product-skill-family-package-name-invalid",
        ));
    }
    let plugin_digest = plugin_entry.sha256().to_owned();
    let legacy_read_session = context.begin_read_session().map_err(|_| {
        ProductMigrationError::new("migration-product-skill-family-compatibility-input-unavailable")
    })?;
    for name in &aliases {
        let yaml_path = format!("skills/{name}/{}{}", "agents", "/openai.yaml");
        let yaml = legacy_read_session
            .read_bounded(&legacy_read_session.root().join(&yaml_path), 128 * 1024)
            .ok()
            .and_then(|bytes| String::from_utf8(bytes).ok())
            .ok_or_else(|| {
                ProductMigrationError::new(
                    "migration-product-skill-family-compatibility-input-invalid",
                )
            })?;
        compatibility_entries.push(SkillArtifact {
            name: name.clone(),
            path: yaml_path,
            openai_yaml: yaml,
            enabled: true,
        });
    }
    legacy_read_session.revalidate().map_err(|_| {
        ProductMigrationError::new("migration-product-skill-family-compatibility-input-mutated")
    })?;
    let package = SkillPackage {
        plugin: HARNESS_PLUGIN.to_owned(),
        version: plugin_version.clone(),
        plugin_digest,
        package_digest: snapshot.package_sha256().to_owned(),
        candidate_id: artifact.candidate_id().to_owned(),
        config_digest: artifact.catalog_id().to_owned(),
        selected: true,
        skills: compatibility_entries,
    };
    let packaged_paths = snapshot
        .entries()
        .iter()
        .map(|entry| entry.path().to_owned())
        .collect::<BTreeSet<_>>();
    Ok(PreparedSkillFamily {
        candidate_id: artifact.candidate_id().to_owned(),
        catalog_id: artifact.catalog_id().to_owned(),
        source_snapshot_id: artifact.source_snapshot_id().to_owned(),
        package_sha256: snapshot.package_sha256().to_owned(),
        plugin_version,
        canonical,
        aliases,
        package,
        packaged_paths,
    })
}

pub(crate) fn make_profile(
    prepared: &PreparedSkillFamily,
    selected_skills: Vec<String>,
) -> SkillProfile {
    SkillProfile {
        plugin: HARNESS_PLUGIN.to_owned(),
        name: "n14-current-candidate".to_owned(),
        digest: digest::bytes(
            &serde_json::to_vec(&("n14-profile-v1", &selected_skills))
                .expect("static profile evidence is serializable"),
        ),
        candidate_id: prepared.candidate_id.clone(),
        config_digest: prepared.catalog_id.clone(),
        plugin_version: prepared.plugin_version.clone(),
        plugin_digest: prepared.package.plugin_digest.clone(),
        selected_skills,
    }
}

fn entry<'a>(snapshot: &'a PackageSnapshot, path: &str) -> Option<&'a PackageEntry> {
    snapshot.entries().iter().find(|entry| entry.path() == path)
}

pub(super) fn valid_digest(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

pub(super) fn projection_digest(value: &SkillFamilyActivationProjection) -> String {
    digest::bytes(
        &serde_json::to_vec(&(
            "n14-skill-family-activation-v1",
            value.candidate_id(),
            value.catalog_id(),
            value.source_snapshot_id(),
            value.package_sha256(),
            value.plugin_version(),
            value.canonical_skill_names(),
            value.excluded_legacy_names(),
            value.profile_rejected_legacy(),
            value.implicit_gateways(),
            value.catalog_exclusion_sha256(),
            value.profile_rejection_sha256(),
            value.implicit_gateway_sha256(),
        ))
        .expect("activation projection is serializable"),
    )
}
