use super::ObjectIdentity;
use crate::distribution::error::{DistributionError, DistributionErrorId, error};
use std::path::Path;

pub(crate) struct Root;

pub(crate) fn open_root(_path: &Path) -> Result<Root, DistributionError> {
    Err(error(DistributionErrorId::CapabilityMismatch))
}

pub(crate) fn read(
    _root: &Root,
    _path: &str,
    _maximum: usize,
) -> Result<(Vec<u8>, ObjectIdentity), DistributionError> {
    Err(error(DistributionErrorId::CapabilityMismatch))
}
