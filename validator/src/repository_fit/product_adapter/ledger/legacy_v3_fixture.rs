use super::*;

#[cfg(all(test, target_vendor = "apple"))]
pub(crate) fn write_legacy_v3_device_number_for_test(
    root: &std::path::Path,
    replacement_device: u64,
) -> Result<(), LedgerError> {
    supported::write_legacy_v3_device_number_for_test(root, replacement_device)
}

#[cfg(all(test, target_vendor = "apple"))]
pub(crate) fn simulate_durable_identity_drift_for_test(
    root: &std::path::Path,
    replacement_inode: u64,
) -> Result<(), LedgerError> {
    supported::simulate_durable_identity_drift_for_test(root, replacement_inode)
}
