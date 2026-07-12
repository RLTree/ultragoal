use crate::distribution::cache::{CacheExpectation, CacheSnapshot, reconcile_cache_read_only};
use crate::distribution::error::{DistributionError, DistributionErrorId, error};

const CACHE_LIMIT: usize = 4 * 1024 * 1024;

pub trait CacheReader {
    fn read_cache(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()>;
}

impl CacheReader for crate::distribution::filesystem::ScopedFile {
    fn read_cache(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        self.inspect(maximum).map_err(|_| ())
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
