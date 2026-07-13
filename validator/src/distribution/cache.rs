use crate::distribution::error::{DistributionError, DistributionErrorId, error};
use crate::distribution::host::MarketplaceTransaction;
use crate::distribution::json;
use crate::distribution::marketplace::{
    MarketplaceExpectation, MarketplacePlan, MarketplaceSnapshot, MarketplaceVerdict, snapshot,
};
use crate::distribution::reader::sha256;
use crate::distribution::reader::validate_relative_path;
use crate::distribution::spec::digest;
use crate::plugin_manifest::Version;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

const CACHE_OBSERVATION_LIMIT: usize = 4 * 1024 * 1024;
const MARKETPLACE_LIMIT: usize = 1024 * 1024;

pub trait MarketplaceEffects {
    fn read(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()>;
    fn compare_exchange(
        &mut self,
        expected_sha256: Option<&str>,
        replacement: Option<&[u8]>,
    ) -> Result<bool, ()>;
}

pub fn apply_marketplace(
    plan: &MarketplacePlan,
    effects: &mut impl MarketplaceEffects,
) -> Result<MarketplaceTransaction, DistributionError> {
    let before = read_marketplace(effects)?;
    if before.as_deref().map(sha256).as_deref() != plan.expected_sha256() {
        return Err(error(DistributionErrorId::InstallConflict));
    }
    match effects.compare_exchange(plan.expected_sha256(), Some(plan.replacement())) {
        Ok(true) => {}
        Ok(false) => return Err(error(DistributionErrorId::InstallConflict)),
        Err(()) => return Err(error(DistributionErrorId::EffectFailed)),
    }
    let after = match read_marketplace(effects) {
        Ok(value) => value,
        Err(failure) => {
            restore_marketplace(plan, effects)?;
            return Err(failure);
        }
    };
    if after.as_deref() != Some(plan.replacement()) {
        restore_marketplace(plan, effects)?;
        return Err(error(DistributionErrorId::ArchiveMismatch));
    }
    Ok(MarketplaceTransaction::new(
        sha256(plan.replacement()),
        before,
    ))
}

fn read_marketplace(
    effects: &mut impl MarketplaceEffects,
) -> Result<Option<Vec<u8>>, DistributionError> {
    let value = effects
        .read(MARKETPLACE_LIMIT)
        .map_err(|_| error(DistributionErrorId::EffectFailed))?;
    if value
        .as_ref()
        .is_some_and(|bytes| bytes.len() > MARKETPLACE_LIMIT)
    {
        return Err(error(DistributionErrorId::ObjectTooLarge));
    }
    Ok(value)
}

fn restore_marketplace(
    plan: &MarketplacePlan,
    effects: &mut impl MarketplaceEffects,
) -> Result<(), DistributionError> {
    let candidate = sha256(plan.replacement());
    match effects.compare_exchange(Some(&candidate), plan.rollback.as_deref()) {
        Ok(true) => Ok(()),
        Ok(false) => Err(error(DistributionErrorId::InstallConflict)),
        Err(()) => Err(error(DistributionErrorId::RollbackFailed)),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyCatalog {
    schema: String,
    context_id: String,
    candidate_id: String,
    scope: crate::distribution::marketplace::MarketplaceScope,
    plugins: Vec<LegacyPlugin>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyPlugin {
    plugin_id: String,
    version: String,
    origin: String,
    package_sha256: String,
}

pub fn verify_marketplace(
    bytes: &[u8],
    expected: &MarketplaceExpectation,
) -> Result<MarketplaceSnapshot, DistributionError> {
    let catalog: LegacyCatalog = json::parse(bytes, 1024 * 1024)?;
    if catalog.schema != "harness-ultragoal.marketplace-catalog.v1"
        || catalog.context_id != expected.context_id
        || catalog.candidate_id != expected.candidate_id
        || catalog.scope != expected.scope
        || catalog.plugins.is_empty()
        || catalog.plugins.len() > 1024
    {
        return Err(error(DistributionErrorId::InvalidSpec));
    }
    let mut seen = BTreeSet::new();
    let mut matching = Vec::new();
    for row in catalog.plugins {
        validate_relative_path(&row.origin)?;
        if Version::parse(&row.version).is_none()
            || !digest(&row.package_sha256)
            || !seen.insert((row.plugin_id.clone(), row.version.clone()))
        {
            return Err(error(DistributionErrorId::InstallConflict));
        }
        if row.plugin_id == expected.plugin_id {
            matching.push(row);
        }
    }
    if matching.len() != 1
        || matching[0].version != expected.version
        || matching[0].origin != expected.origin
        || matching[0].package_sha256 != expected.package_sha256
    {
        return Err(error(DistributionErrorId::InstallConflict));
    }
    Ok(snapshot(
        expected,
        MarketplaceVerdict::Verified,
        Some(sha256(bytes)),
    ))
}

pub fn unavailable_marketplace(
    expected: &MarketplaceExpectation,
    reason_id: &str,
) -> Result<MarketplaceSnapshot, DistributionError> {
    if reason_id.is_empty()
        || reason_id.len() > 96
        || !reason_id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return Err(error(DistributionErrorId::InvalidSpec));
    }
    Ok(snapshot(expected, MarketplaceVerdict::Unavailable, None))
}

#[derive(Clone, Debug)]
pub struct CacheExpectation {
    context_id: String,
    candidate_id: String,
    cache_root_id: String,
    marketplace: String,
    plugin_id: String,
    version: String,
    package_tree_sha256: String,
}

impl CacheExpectation {
    pub fn new(
        context_id: String,
        candidate_id: String,
        cache_root_id: String,
        marketplace: String,
        plugin_id: String,
        version: String,
        package_tree_sha256: String,
    ) -> Result<Self, DistributionError> {
        if !digest(&context_id)
            || !digest(&candidate_id)
            || !digest(&cache_root_id)
            || !safe_name(&marketplace)
            || plugin_id != "harness-ultragoal"
            || Version::parse(&version).is_none()
            || !digest(&package_tree_sha256)
        {
            return Err(error(DistributionErrorId::InvalidSpec));
        }
        Ok(Self {
            context_id,
            candidate_id,
            cache_root_id,
            marketplace,
            plugin_id,
            version,
            package_tree_sha256,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CacheSnapshot {
    context_id: String,
    candidate_id: String,
    cache_root_id: String,
    marketplace: String,
    plugin_id: String,
    version: String,
    observation_sha256: String,
    package_tree_sha256: String,
}

impl CacheSnapshot {
    pub fn context_id(&self) -> &str {
        &self.context_id
    }
    pub fn candidate_id(&self) -> &str {
        &self.candidate_id
    }
    pub fn cache_root_id(&self) -> &str {
        &self.cache_root_id
    }
    pub fn marketplace(&self) -> &str {
        &self.marketplace
    }
    pub fn plugin_id(&self) -> &str {
        &self.plugin_id
    }
    pub fn version(&self) -> &str {
        &self.version
    }
    pub fn observation_sha256(&self) -> &str {
        &self.observation_sha256
    }
    pub fn package_tree_sha256(&self) -> &str {
        &self.package_tree_sha256
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CacheObservation {
    schema: String,
    context_id: String,
    candidate_id: String,
    cache_root_id: String,
    entries: Vec<CacheEntry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CacheEntry {
    marketplace: String,
    plugin_id: String,
    version: String,
    package_tree_sha256: String,
}

pub fn reconcile_cache_read_only(
    bytes: &[u8],
    expectation: &CacheExpectation,
) -> Result<CacheSnapshot, DistributionError> {
    let observation: CacheObservation = json::parse(bytes, CACHE_OBSERVATION_LIMIT)?;
    if observation.schema != "harness-ultragoal.codex-cache-observation.v1"
        || observation.context_id != expectation.context_id
        || observation.candidate_id != expectation.candidate_id
        || observation.cache_root_id != expectation.cache_root_id
        || observation.entries.len() > 4096
    {
        return Err(error(DistributionErrorId::InvalidSpec));
    }
    let mut identities = BTreeSet::new();
    let mut matched = None;
    let mut plugin_versions = 0usize;
    for row in observation.entries {
        if !safe_name(&row.marketplace)
            || !safe_name(&row.plugin_id)
            || Version::parse(&row.version).is_none()
            || !digest(&row.package_tree_sha256)
            || !identities.insert((
                row.marketplace.clone(),
                row.plugin_id.clone(),
                row.version.clone(),
            ))
        {
            return Err(error(DistributionErrorId::InstallConflict));
        }
        if row.plugin_id == expectation.plugin_id {
            plugin_versions += 1;
            if row.marketplace == expectation.marketplace && row.version == expectation.version {
                matched = Some(row);
            }
        }
    }
    let matched = matched.ok_or_else(|| error(DistributionErrorId::InstallConflict))?;
    if plugin_versions != 1 || matched.package_tree_sha256 != expectation.package_tree_sha256 {
        return Err(error(DistributionErrorId::InstallConflict));
    }
    Ok(CacheSnapshot {
        context_id: observation.context_id,
        candidate_id: observation.candidate_id,
        cache_root_id: observation.cache_root_id,
        marketplace: matched.marketplace,
        plugin_id: matched.plugin_id,
        version: matched.version,
        observation_sha256: sha256(bytes),
        package_tree_sha256: matched.package_tree_sha256,
    })
}

fn safe_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
        })
}
