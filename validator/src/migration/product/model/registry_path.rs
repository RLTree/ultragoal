pub(super) const MAX_PRODUCT_ITEMS: usize = 4_096;
pub(super) const MAX_MACHINE_OUTPUT_BYTES: usize = 2 * 1024 * 1024;
pub(super) const MAX_APPLY_TTL_MS: u64 = 10 * 60 * 1_000;
const MAX_COMPATIBILITY_WARNING_BYTES: usize = 512;
const MAX_COMPATIBILITY_MEASUREMENT_WINDOW_MS: u64 = 90 * 24 * 60 * 60 * 1_000;
const MAX_COMPATIBILITY_DEADLINE_HORIZON_MS: u64 = 366 * 24 * 60 * 60 * 1_000;
const MAX_COMPATIBILITY_OBSERVED_INVOCATIONS: u64 = 1_000_000_000;
const MAX_COMPATIBILITY_CONSECUTIVE_WINDOWS: u16 = 52;
pub(super) const REQUIRED_FALSE_PASS_CONTROLS: [&str; 5] = [
    "proof-artifact",
    "receipt-production",
    "score-only",
    "test-manipulation",
    "verbosity",
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ProductMigrationError {
    code: &'static str,
}

impl ProductMigrationError {
    pub(crate) const fn new(code: &'static str) -> Self {
        Self { code }
    }

    pub(crate) const fn code(&self) -> &'static str {
        self.code
    }
}

impl fmt::Display for ProductMigrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code)
    }
}

impl std::error::Error for ProductMigrationError {}

/// Root-owned authority for both single-use migration authorization and
/// compatibility-boundary observations. Registry bytes can declare a boundary
/// policy, but only this authority can supply the current time and product
/// version used to decide whether that policy remains open.
pub(crate) trait ApplyAuthorizationAuthority {
    fn principal_id(&self) -> &str;
    fn authority_id(&self) -> &str;
    fn session_id(&self) -> &str;
    fn nonce_sha256(&self) -> &str;
    fn issued_at_unix_ms(&self) -> u64;
    fn expires_at_unix_ms(&self) -> u64;
    fn now_unix_ms(&self) -> u64;
    fn current_binding(&self) -> (&str, &str);
    fn seal(&mut self, binding_sha256: &str) -> Result<String, ProductMigrationError>;
    fn verify_seal(&self, binding_sha256: &str, seal_sha256: &str) -> bool;

    fn compatibility_boundary_authority_id(&self) -> &str;
    fn compatibility_boundary_source_identity_sha256(&self) -> &str;
    fn compatibility_boundary_observation_sequence(&self) -> u64;
    fn compatibility_boundary_observed_at_unix_ms(&self) -> u64;
    fn compatibility_boundary_current_product_version(&self) -> &str;
    fn seal_compatibility_boundary(
        &self,
        binding_sha256: &str,
    ) -> Result<String, ProductMigrationError>;
    fn verify_compatibility_boundary_seal(&self, binding_sha256: &str, seal_sha256: &str) -> bool;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct AdoptedRegistrySnapshot {
    relative_path: String,
    file_kind: SurfaceFileKind,
    link_count: u64,
    source_identity_sha256: String,
    bytes: Vec<u8>,
    bytes_sha256: String,
}

impl AdoptedRegistrySnapshot {
    pub(crate) fn observed(
        relative_path: impl Into<String>,
        file_kind: SurfaceFileKind,
        link_count: u64,
        source_identity_sha256: impl Into<String>,
        bytes: Vec<u8>,
    ) -> Result<Self, ProductMigrationError> {
        let relative_path = relative_path.into();
        let source_identity_sha256 = source_identity_sha256.into();
        let bytes_sha256 = digest(&bytes);
        let value = Self {
            relative_path,
            file_kind,
            link_count,
            source_identity_sha256,
            bytes,
            bytes_sha256,
        };
        value.validate()?;
        Ok(value)
    }

    pub(crate) fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub(super) fn validate(&self) -> Result<(), ProductMigrationError> {
        if self.relative_path != MIGRATION_REGISTRY_PATH
            || !self.relative_path.is_ascii()
            || !safe_relative_path(&self.relative_path)
        {
            return Err(ProductMigrationError::new(
                "migration-product-registry-path-refused",
            ));
        }
        if self.file_kind != SurfaceFileKind::Regular || self.link_count != 1 {
            return Err(ProductMigrationError::new(
                "migration-product-registry-file-refused",
            ));
        }
        if self.bytes.is_empty() || self.bytes.len() as u64 > MAX_MIGRATION_REGISTRY_BYTES {
            return Err(ProductMigrationError::new(
                "migration-product-registry-size-refused",
            ));
        }
        if !valid_sha256(&self.source_identity_sha256)
            || !valid_sha256(&self.bytes_sha256)
            || digest(&self.bytes) != self.bytes_sha256
        {
            return Err(ProductMigrationError::new(
                "migration-product-registry-observation-mutated",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MigrationInputBinding {
    schema_version: String,
    live_context_id: String,
    candidate_id: String,
    catalog_id: String,
    read_session_id: String,
    inventory_sha256: String,
    registry_sha256: String,
    registry_source_identity_sha256: String,
    binding_sha256: String,
}
