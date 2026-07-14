const CACHE_OBSERVATION_LIMIT: usize = 4 * 1024 * 1024;

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
        if !safe_cache_name(&row.marketplace)
            || !safe_cache_name(&row.plugin_id)
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
