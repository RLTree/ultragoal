use super::{DistributionError, ScopedFile};

impl ScopedFile {
    /// Atomically publishes a regular executable whose bytes have already
    /// been authenticated by the caller's package boundary.
    #[cfg(unix)]
    pub fn apply_executable(
        &self,
        expected_sha256: Option<&str>,
        replacement: Option<&[u8]>,
    ) -> Result<bool, DistributionError> {
        self.apply_with_mode(expected_sha256, replacement, 0o755)
    }

    #[cfg(not(unix))]
    pub fn apply_executable(
        &self,
        _expected_sha256: Option<&str>,
        _replacement: Option<&[u8]>,
    ) -> Result<bool, DistributionError> {
        Err(super::error(super::DistributionErrorId::CapabilityMismatch))
    }
}
