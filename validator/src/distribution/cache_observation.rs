use crate::distribution::cache::{CacheExpectation, CacheSnapshot, reconcile_cache_read_only};
use crate::distribution::error::{DistributionError, DistributionErrorId, error};
use crate::distribution::reader::sha256;
use serde::Serialize;

const CACHE_LIMIT: usize = 4 * 1024 * 1024;

pub trait CacheReader {
    fn read_cache(
        &mut self,
        maximum: usize,
    ) -> Result<Option<Vec<u8>>, crate::distribution::EffectFailure>;
}

impl CacheReader for crate::distribution::filesystem::ScopedFile {
    fn read_cache(
        &mut self,
        maximum: usize,
    ) -> Result<Option<Vec<u8>>, crate::distribution::EffectFailure> {
        self.inspect(maximum)
            .map_err(|_| crate::distribution::EffectFailure)
    }
}

pub fn reconcile_cache_file(
    reader: &mut impl CacheReader,
    expectation: &CacheExpectation,
) -> Result<CacheSnapshot, DistributionError> {
    let before = reader
        .read_cache(CACHE_LIMIT)
        .map_err(|_| error(DistributionErrorId::ObjectUnavailable))?
        .ok_or_else(|| error(DistributionErrorId::ObjectUnavailable))?;
    let snapshot = reconcile_cache_read_only(&before, expectation)?;
    let after = reader
        .read_cache(CACHE_LIMIT)
        .map_err(|_| error(DistributionErrorId::ObjectUnavailable))?
        .ok_or_else(|| error(DistributionErrorId::ObjectUnavailable))?;
    if before != after {
        return Err(error(DistributionErrorId::ObjectChanged));
    }
    Ok(snapshot)
}

/// Publishes one isolated-host cache row and immediately re-observes it. This
/// is deliberately separate from package installation: a cache row is not
/// install proof and a later reader must authenticate it again.
pub fn publish_cache_file(
    file: &crate::distribution::filesystem::ScopedFile,
    expectation: &CacheExpectation,
) -> Result<(), DistributionError> {
    if file.root_id() != expectation.cache_root_id() {
        return Err(error(DistributionErrorId::ProvenanceMismatch));
    }
    #[derive(Serialize)]
    struct Entry<'a> {
        marketplace: &'a str,
        plugin_id: &'a str,
        version: &'a str,
        package_tree_sha256: &'a str,
    }
    #[derive(Serialize)]
    struct Document<'a> {
        schema: &'static str,
        context_id: &'a str,
        candidate_id: &'a str,
        cache_root_id: &'a str,
        entries: [Entry<'a>; 1],
    }
    let bytes = serde_json::to_vec(&Document {
        schema: "harness-ultragoal.codex-cache-observation.v1",
        context_id: expectation.context_id(),
        candidate_id: expectation.candidate_id(),
        cache_root_id: expectation.cache_root_id(),
        entries: [Entry {
            marketplace: expectation.marketplace(),
            plugin_id: expectation.plugin_id(),
            version: expectation.version(),
            package_tree_sha256: expectation.package_tree_sha256(),
        }],
    })
    .map_err(|_| error(DistributionErrorId::InvalidSpec))?;
    let current = file.inspect(CACHE_LIMIT)?;
    if current.as_deref() != Some(bytes.as_slice())
        && !file.apply(current.as_deref().map(sha256).as_deref(), Some(&bytes))?
    {
        return Err(error(DistributionErrorId::InstallConflict));
    }
    let observed = file
        .inspect(CACHE_LIMIT)?
        .ok_or_else(|| error(DistributionErrorId::ObjectUnavailable))?;
    if observed != bytes {
        return Err(error(DistributionErrorId::ObjectChanged));
    }
    Ok(())
}
