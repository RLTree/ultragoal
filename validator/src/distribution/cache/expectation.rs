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
            || !safe_cache_name(&marketplace)
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

fn safe_cache_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
        })
}
