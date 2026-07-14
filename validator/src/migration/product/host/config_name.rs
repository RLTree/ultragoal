const CONFIG_NAME: &str = "authority.json";
const KEY_NAME: &str = "secret.key";
const LOCK_NAME: &str = "migration.lock";
const LEDGER_NAME: &str = "state.json";
const CONFIG_SCHEMA: &str = "DarwinMigrationHostAuthority-v1";
const HOST_SCOPE_DOMAIN: &str = "harness-ultragoal.migration-host-scope.v1";
const BOUNDARY_SOURCE_DOMAIN: &str = "harness-ultragoal.migration-boundary-source.v1";
const MAX_CONFIG_BYTES: u64 = 64 * 1024;
const MAX_KEY_BYTES: u64 = 64;
pub(super) const MAX_HOST_FILE_BYTES: u64 = 8 * 1024 * 1024;
pub(super) const MAX_SOURCE_FILE_BYTES: u64 = 8 * 1024 * 1024;
const KEY_BYTES: usize = 32;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct HostError {
    code: &'static str,
}

impl HostError {
    pub(crate) const fn new(code: &'static str) -> Self {
        Self { code }
    }

    pub(crate) const fn code(&self) -> &'static str {
        self.code
    }

    fn product(self) -> ProductMigrationError {
        ProductMigrationError::new(self.code)
    }
}

impl fmt::Display for HostError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code)
    }
}

impl std::error::Error for HostError {}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct HostAuthorityConfig {
    schema_version: String,
    repository_scope_sha256: String,
    candidate_id: String,
    product_version: String,
    principal_id: String,
    authorization_authority_id: String,
    compatibility_boundary_authority_id: String,
    compatibility_boundary_source_identity_sha256: String,
    clock_anchor_unix_ms: u64,
    clock_anchor_monotonic_ns: u64,
    config_sha256: String,
}

pub(super) struct HostAuthorityConfigDefinition {
    pub(super) repository_scope_sha256: String,
    pub(super) candidate_id: String,
    pub(super) product_version: String,
    pub(super) clock_anchor_unix_ms: u64,
    pub(super) clock_anchor_monotonic_ns: u64,
}

impl HostAuthorityConfig {
    fn issue(definition: HostAuthorityConfigDefinition) -> Result<Self, HostError> {
        let HostAuthorityConfigDefinition {
            repository_scope_sha256,
            candidate_id,
            product_version,
            clock_anchor_unix_ms,
            clock_anchor_monotonic_ns,
        } = definition;
        let principal_id = "migration-host-operator".to_owned();
        let authorization_authority_id = "migration-host-authorization-authority".to_owned();
        let compatibility_boundary_authority_id =
            "migration-host-compatibility-boundary-authority".to_owned();
        let compatibility_boundary_source_identity_sha256 = digest_bytes(
            format!(
                "{BOUNDARY_SOURCE_DOMAIN}|{repository_scope_sha256}|{candidate_id}|{product_version}"
            )
            .as_bytes(),
        );
        let mut value = Self {
            schema_version: CONFIG_SCHEMA.to_owned(),
            repository_scope_sha256,
            candidate_id,
            product_version,
            principal_id,
            authorization_authority_id,
            compatibility_boundary_authority_id,
            compatibility_boundary_source_identity_sha256,
            clock_anchor_unix_ms,
            clock_anchor_monotonic_ns,
            config_sha256: String::new(),
        };
        value.config_sha256 = config_digest(&value);
        value.validate()?;
        Ok(value)
    }

    fn validate(&self) -> Result<(), HostError> {
        if self.schema_version != CONFIG_SCHEMA
            || !super::super::valid_sha256(&self.repository_scope_sha256)
            || !super::super::valid_sha256(&self.candidate_id)
            || !valid_product_version(&self.product_version)
            || !super::super::valid_identifier(&self.principal_id)
            || !super::super::valid_identifier(&self.authorization_authority_id)
            || !super::super::valid_identifier(&self.compatibility_boundary_authority_id)
            || !super::super::valid_sha256(&self.compatibility_boundary_source_identity_sha256)
            || self.clock_anchor_unix_ms == 0
            || self.clock_anchor_monotonic_ns == 0
            || self.config_sha256 != config_digest(self)
        {
            return Err(HostError::new("migration-host-authority-config-invalid"));
        }
        Ok(())
    }

    fn canonical_bytes(&self) -> Result<Vec<u8>, HostError> {
        serde_json::to_vec(self)
            .map_err(|_| HostError::new("migration-host-authority-config-invalid"))
    }
}

pub(super) struct HostContext {
    repository: AnchoredDirectory,
    state: AnchoredDirectory,
    config: HostAuthorityConfig,
    config_bytes: Vec<u8>,
    config_identity: FileIdentity,
    key: [u8; KEY_BYTES],
    key_bytes: Vec<u8>,
    key_identity: FileIdentity,
    lock_identity: FileIdentity,
    _process_lock: ProcessLock,
    io: Mutex<()>,
    #[cfg(test)]
    trusted_time_advance_ms: std::sync::atomic::AtomicU64,
    #[cfg(test)]
    fail_on_cas: Mutex<Option<usize>>,
    #[cfg(test)]
    cas_count: std::sync::atomic::AtomicUsize,
}
