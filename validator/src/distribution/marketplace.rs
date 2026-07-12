use crate::distribution::error::{DistributionError, DistributionErrorId, error};
use crate::distribution::json;
use crate::distribution::model::PackageIdentity;
use crate::distribution::reader::{sha256, validate_relative_path};
use crate::distribution::spec::digest;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub use crate::distribution::cache::{MarketplaceEffects, apply_marketplace};
pub use crate::distribution::cache::{unavailable_marketplace, verify_marketplace};
pub use crate::distribution::host::{MarketplaceTransaction, rollback_marketplace};

const MARKETPLACE_LIMIT: usize = 1024 * 1024;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum MarketplaceScope {
    Repository,
    Personal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum MarketplaceVerdict {
    Verified,
    Declared,
    Unavailable,
}

#[derive(Clone, Debug)]
pub struct MarketplaceExpectation {
    pub(crate) context_id: String,
    pub(crate) candidate_id: String,
    pub(crate) scope: MarketplaceScope,
    pub(crate) plugin_id: String,
    pub(crate) version: String,
    pub(crate) origin: String,
    pub(crate) package_sha256: String,
}

impl MarketplaceExpectation {
    pub fn new(
        context_id: String,
        candidate_id: String,
        scope: MarketplaceScope,
        plugin_id: String,
        version: String,
        origin: String,
        package_sha256: String,
    ) -> Result<Self, DistributionError> {
        validate_relative_path(&origin)?;
        if !digest(&context_id)
            || !digest(&candidate_id)
            || plugin_id != "harness-ultragoal"
            || crate::plugin_manifest::Version::parse(&version).is_none()
            || !digest(&package_sha256)
        {
            return Err(error(DistributionErrorId::InvalidSpec));
        }
        Ok(Self {
            context_id,
            candidate_id,
            scope,
            plugin_id,
            version,
            origin,
            package_sha256,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct MarketplaceSnapshot {
    context_id: String,
    candidate_id: String,
    scope: MarketplaceScope,
    verdict: MarketplaceVerdict,
    catalog_sha256: Option<String>,
    package_sha256: String,
}

impl MarketplaceSnapshot {
    pub fn context_id(&self) -> &str {
        &self.context_id
    }
    pub fn candidate_id(&self) -> &str {
        &self.candidate_id
    }
    pub const fn scope(&self) -> MarketplaceScope {
        self.scope
    }
    pub const fn verdict(&self) -> MarketplaceVerdict {
        self.verdict
    }
    pub fn package_sha256(&self) -> &str {
        &self.package_sha256
    }
    pub fn catalog_sha256(&self) -> Option<&str> {
        self.catalog_sha256.as_deref()
    }
}

pub(crate) fn snapshot(
    expected: &MarketplaceExpectation,
    verdict: MarketplaceVerdict,
    catalog_sha256: Option<String>,
) -> MarketplaceSnapshot {
    MarketplaceSnapshot {
        context_id: expected.context_id.clone(),
        candidate_id: expected.candidate_id.clone(),
        scope: expected.scope,
        verdict,
        catalog_sha256,
        package_sha256: expected.package_sha256.clone(),
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CodexMarketplace {
    name: String,
    interface: CodexInterface,
    plugins: Vec<CodexPlugin>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct CodexInterface {
    #[serde(rename = "displayName")]
    display_name: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CodexPlugin {
    name: String,
    source: CodexSource,
    policy: CodexPolicy,
    category: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct CodexSource {
    source: String,
    path: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct CodexPolicy {
    installation: String,
    authentication: String,
}

impl CodexPlugin {
    pub fn harness_ultragoal() -> Self {
        Self {
            name: "harness-ultragoal".into(),
            source: CodexSource {
                source: "local".into(),
                path: "./plugins/harness-ultragoal".into(),
            },
            policy: CodexPolicy {
                installation: "AVAILABLE".into(),
                authentication: "ON_INSTALL".into(),
            },
            category: "Productivity".into(),
        }
    }
}

#[derive(Clone)]
pub struct MarketplacePlan {
    expected_sha256: Option<String>,
    replacement: Vec<u8>,
    pub(crate) rollback: Option<Vec<u8>>,
    package: PackageIdentity,
}

impl MarketplacePlan {
    pub fn replacement(&self) -> &[u8] {
        &self.replacement
    }
    pub fn expected_sha256(&self) -> Option<&str> {
        self.expected_sha256.as_deref()
    }
    pub fn package(&self) -> &PackageIdentity {
        &self.package
    }
}

pub fn plan_codex_marketplace(
    current: Option<&[u8]>,
    expected_sha256: Option<&str>,
    name: &str,
    display_name: &str,
    desired: CodexPlugin,
    package: PackageIdentity,
) -> Result<MarketplacePlan, DistributionError> {
    package.validate()?;
    validate_name(name)?;
    if display_name.is_empty()
        || display_name.len() > 128
        || display_name.bytes().any(|byte| byte.is_ascii_control())
        || expected_sha256.is_some_and(|value| !digest(value))
        || current.map(sha256).as_deref() != expected_sha256
    {
        return Err(error(DistributionErrorId::InstallConflict));
    }
    validate_plugin(&desired)?;
    let mut document = match current {
        Some(bytes) => json::parse::<CodexMarketplace>(bytes, MARKETPLACE_LIMIT)?,
        None => CodexMarketplace {
            name: name.into(),
            interface: CodexInterface {
                display_name: display_name.into(),
            },
            plugins: Vec::new(),
        },
    };
    validate_document(&document)?;
    if document.name != name {
        return Err(error(DistributionErrorId::InstallConflict));
    }
    match document
        .plugins
        .iter()
        .position(|row| row.name == desired.name)
    {
        Some(index) => document.plugins[index] = desired,
        None => document.plugins.push(desired),
    }
    validate_document(&document)?;
    let mut replacement = serde_json::to_vec_pretty(&document)
        .map_err(|_| error(DistributionErrorId::InvalidSpec))?;
    replacement.push(b'\n');
    Ok(MarketplacePlan {
        expected_sha256: expected_sha256.map(str::to_owned),
        replacement,
        rollback: current.map(<[u8]>::to_vec),
        package,
    })
}

fn validate_document(value: &CodexMarketplace) -> Result<(), DistributionError> {
    validate_name(&value.name)?;
    if value.interface.display_name.is_empty()
        || value.interface.display_name.len() > 128
        || value
            .interface
            .display_name
            .bytes()
            .any(|byte| byte.is_ascii_control())
        || value.plugins.len() > 1024
    {
        return Err(error(DistributionErrorId::ObjectTooLarge));
    }
    let mut names = BTreeSet::new();
    let mut paths = BTreeSet::new();
    for row in &value.plugins {
        validate_plugin(row)?;
        if !names.insert(&row.name) || !paths.insert(&row.source.path) {
            return Err(error(DistributionErrorId::InstallConflict));
        }
    }
    Ok(())
}

fn validate_plugin(value: &CodexPlugin) -> Result<(), DistributionError> {
    validate_name(&value.name)?;
    let path = value
        .source
        .path
        .strip_prefix("./")
        .ok_or_else(|| error(DistributionErrorId::InvalidPath))?;
    validate_relative_path(path)?;
    if value.source.source != "local"
        || value.policy.installation != "AVAILABLE"
        || value.policy.authentication != "ON_INSTALL"
        || value.category.is_empty()
        || value.category.len() > 128
        || value.category.bytes().any(|byte| byte.is_ascii_control())
    {
        return Err(error(DistributionErrorId::InvalidSpec));
    }
    Ok(())
}

fn validate_name(value: &str) -> Result<(), DistributionError> {
    if value.is_empty()
        || value.len() > 128
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
        })
    {
        return Err(error(DistributionErrorId::InvalidSpec));
    }
    Ok(())
}
