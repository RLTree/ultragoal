use super::super::{
    digest, safe_reference, safe_relative_path, valid_identifier, valid_sha256,
    valid_stable_identifier, InventorySurface, MigrationInventory, SurfaceFileKind, SurfaceStatus,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub(super) const REGISTRY_PATH: &str = "migration/authority-routes.json";
pub(super) const MAX_REGISTRY_BYTES: usize = 2 * 1024 * 1024;
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

    pub(crate) fn bytes_sha256(&self) -> &str {
        &self.bytes_sha256
    }

    pub(crate) fn source_identity_sha256(&self) -> &str {
        &self.source_identity_sha256
    }

    pub(super) fn validate(&self) -> Result<(), ProductMigrationError> {
        if self.relative_path != REGISTRY_PATH
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
        if self.bytes.is_empty() || self.bytes.len() > MAX_REGISTRY_BYTES {
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

impl MigrationInputBinding {
    pub(super) fn issue(input: &ProductInputSnapshot) -> Self {
        let binding_sha256 = digest(
            format!(
                "migration-product-input-v1|{}|{}|{}|{}|{}|{}|{}",
                input.inventory.live_context_id,
                input.inventory.candidate_id,
                input.inventory.catalog_id,
                input.inventory.read_session_id,
                input.inventory.inventory_sha256,
                input.registry.bytes_sha256,
                input.registry.source_identity_sha256,
            )
            .as_bytes(),
        );
        Self {
            schema_version: "MigrationProductInputBinding-v1".to_owned(),
            live_context_id: input.inventory.live_context_id.clone(),
            candidate_id: input.inventory.candidate_id.clone(),
            catalog_id: input.inventory.catalog_id.clone(),
            read_session_id: input.inventory.read_session_id.clone(),
            inventory_sha256: input.inventory.inventory_sha256.clone(),
            registry_sha256: input.registry.bytes_sha256.clone(),
            registry_source_identity_sha256: input.registry.source_identity_sha256.clone(),
            binding_sha256,
        }
    }

    pub(crate) fn live_context_id(&self) -> &str {
        &self.live_context_id
    }

    pub(crate) fn candidate_id(&self) -> &str {
        &self.candidate_id
    }

    pub(crate) fn catalog_id(&self) -> &str {
        &self.catalog_id
    }

    pub(crate) fn read_session_id(&self) -> &str {
        &self.read_session_id
    }

    pub(crate) fn inventory_sha256(&self) -> &str {
        &self.inventory_sha256
    }

    pub(crate) fn registry_sha256(&self) -> &str {
        &self.registry_sha256
    }

    pub(crate) fn binding_sha256(&self) -> &str {
        &self.binding_sha256
    }

    pub(super) fn validate(&self) -> bool {
        self.schema_version == "MigrationProductInputBinding-v1"
            && valid_sha256(&self.live_context_id)
            && valid_sha256(&self.candidate_id)
            && valid_sha256(&self.catalog_id)
            && valid_sha256(&self.read_session_id)
            && valid_sha256(&self.inventory_sha256)
            && valid_sha256(&self.registry_sha256)
            && valid_sha256(&self.registry_source_identity_sha256)
            && self.binding_sha256
                == digest(
                    format!(
                        "migration-product-input-v1|{}|{}|{}|{}|{}|{}|{}",
                        self.live_context_id,
                        self.candidate_id,
                        self.catalog_id,
                        self.read_session_id,
                        self.inventory_sha256,
                        self.registry_sha256,
                        self.registry_source_identity_sha256,
                    )
                    .as_bytes(),
                )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ProductInputSnapshot {
    inventory: MigrationInventory,
    registry: AdoptedRegistrySnapshot,
}

impl ProductInputSnapshot {
    pub(crate) fn observed(
        inventory: MigrationInventory,
        registry: AdoptedRegistrySnapshot,
    ) -> Result<Self, ProductMigrationError> {
        let value = Self {
            inventory,
            registry,
        };
        value.validate()?;
        Ok(value)
    }

    pub(crate) fn inventory(&self) -> &MigrationInventory {
        &self.inventory
    }

    pub(crate) fn registry(&self) -> &AdoptedRegistrySnapshot {
        &self.registry
    }

    pub(crate) fn binding(&self) -> MigrationInputBinding {
        MigrationInputBinding::issue(self)
    }

    pub(super) fn validate(&self) -> Result<(), ProductMigrationError> {
        self.inventory
            .validate()
            .map_err(|_| ProductMigrationError::new("migration-product-inventory-refused"))?;
        self.registry.validate()?;
        validate_inventory_paths(&self.inventory)
    }
}

pub(crate) trait MigrationInputSource {
    /// Captures the exact current semantic inventory and adopted registry bytes
    /// in one root-owned read session.
    fn capture(&mut self) -> Result<ProductInputSnapshot, ProductMigrationError>;

    /// Revalidates the original handles and registry bytes. `applied_effect_ids`
    /// are the only semantic mutations the implementation may exempt.
    fn revalidate(
        &mut self,
        binding: &MigrationInputBinding,
        applied_effect_ids: &[String],
    ) -> Result<(), ProductMigrationError>;
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PlanDisposition {
    PendingMigration,
    AdoptCompatibility,
    RetireAuthority,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PhysicalBytesPolicy {
    Preserve,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CompatibilityBoundaryObservation {
    schema_version: String,
    authority_id: String,
    source_identity_sha256: String,
    observation_sequence: u64,
    observed_at_unix_ms: u64,
    current_product_version: String,
    binding_sha256: String,
    seal_sha256: String,
    observation_sha256: String,
}

impl CompatibilityBoundaryObservation {
    fn issue(
        authority_id: String,
        source_identity_sha256: String,
        observation_sequence: u64,
        observed_at_unix_ms: u64,
        current_product_version: String,
        binding_sha256: String,
        seal_sha256: String,
    ) -> Result<Self, ProductMigrationError> {
        let observation_sha256 = digest(
            format!(
                "migration-compatibility-boundary-observation-v1|{}|{}",
                binding_sha256, seal_sha256
            )
            .as_bytes(),
        );
        let value = Self {
            schema_version: "CompatibilityBoundaryObservation-v1".to_owned(),
            authority_id,
            source_identity_sha256,
            observation_sequence,
            observed_at_unix_ms,
            current_product_version,
            binding_sha256,
            seal_sha256,
            observation_sha256,
        };
        if !value.validate() {
            return Err(ProductMigrationError::new(
                "migration-product-compatibility-boundary-observation-invalid",
            ));
        }
        Ok(value)
    }

    pub(super) fn observation_sha256(&self) -> &str {
        &self.observation_sha256
    }

    pub(super) fn validate(&self) -> bool {
        self.schema_version == "CompatibilityBoundaryObservation-v1"
            && valid_identifier(&self.authority_id)
            && valid_sha256(&self.source_identity_sha256)
            && parse_product_version(&self.current_product_version).is_some()
            && self.binding_sha256
                == compatibility_boundary_observation_binding(
                    &self.authority_id,
                    &self.source_identity_sha256,
                    self.observation_sequence,
                    self.observed_at_unix_ms,
                    &self.current_product_version,
                )
            && valid_sha256(&self.seal_sha256)
            && self.observation_sha256
                == digest(
                    format!(
                        "migration-compatibility-boundary-observation-v1|{}|{}",
                        self.binding_sha256, self.seal_sha256
                    )
                    .as_bytes(),
                )
    }

    pub(super) fn is_same_source_and_monotonic_after(&self, prior: &Self) -> bool {
        if !self.validate()
            || !prior.validate()
            || self.authority_id != prior.authority_id
            || self.source_identity_sha256 != prior.source_identity_sha256
            || self.observation_sequence < prior.observation_sequence
            || self.observed_at_unix_ms < prior.observed_at_unix_ms
        {
            return false;
        }
        let Some(current) = parse_product_version(&self.current_product_version) else {
            return false;
        };
        let Some(previous) = parse_product_version(&prior.current_product_version) else {
            return false;
        };
        if current < previous {
            return false;
        }
        let state_changed = self.observed_at_unix_ms != prior.observed_at_unix_ms
            || self.current_product_version != prior.current_product_version;
        !state_changed || self.observation_sequence > prior.observation_sequence
    }

    pub(super) fn seal_verified_by(&self, authority: &dyn ApplyAuthorizationAuthority) -> bool {
        self.validate()
            && authority.compatibility_boundary_authority_id() == self.authority_id
            && authority.compatibility_boundary_source_identity_sha256()
                == self.source_identity_sha256
            && authority.verify_compatibility_boundary_seal(&self.binding_sha256, &self.seal_sha256)
    }
}

pub(super) fn capture_compatibility_boundary_observation(
    authority: &dyn ApplyAuthorizationAuthority,
) -> Result<CompatibilityBoundaryObservation, ProductMigrationError> {
    let authority_id = authority.compatibility_boundary_authority_id().to_owned();
    let source_identity_sha256 = authority
        .compatibility_boundary_source_identity_sha256()
        .to_owned();
    let observation_sequence = authority.compatibility_boundary_observation_sequence();
    let observed_at_unix_ms = authority.compatibility_boundary_observed_at_unix_ms();
    let current_product_version = authority
        .compatibility_boundary_current_product_version()
        .to_owned();
    let binding_sha256 = compatibility_boundary_observation_binding(
        &authority_id,
        &source_identity_sha256,
        observation_sequence,
        observed_at_unix_ms,
        &current_product_version,
    );
    let seal_sha256 = authority.seal_compatibility_boundary(&binding_sha256)?;
    if authority.compatibility_boundary_authority_id() != authority_id
        || authority.compatibility_boundary_source_identity_sha256() != source_identity_sha256
        || authority.compatibility_boundary_observation_sequence() != observation_sequence
        || authority.compatibility_boundary_observed_at_unix_ms() != observed_at_unix_ms
        || authority.compatibility_boundary_current_product_version() != current_product_version
        || !authority.verify_compatibility_boundary_seal(&binding_sha256, &seal_sha256)
    {
        return Err(ProductMigrationError::new(
            "migration-product-compatibility-boundary-authority-refused",
        ));
    }
    CompatibilityBoundaryObservation::issue(
        authority_id,
        source_identity_sha256,
        observation_sequence,
        observed_at_unix_ms,
        current_product_version,
        binding_sha256,
        seal_sha256,
    )
}

fn compatibility_boundary_observation_binding(
    authority_id: &str,
    source_identity_sha256: &str,
    observation_sequence: u64,
    observed_at_unix_ms: u64,
    current_product_version: &str,
) -> String {
    digest(
        format!(
            "migration-compatibility-boundary-observation-binding-v1|{}|{}|{}|{}|{}",
            authority_id,
            source_identity_sha256,
            observation_sequence,
            observed_at_unix_ms,
            current_product_version,
        )
        .as_bytes(),
    )
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CompatibilityUsageMeasurement {
    schema_version: String,
    route_id: String,
    metric: String,
    evidence_sha256: String,
    window_start_unix_ms: u64,
    window_end_unix_ms: u64,
    observed_invocations: u64,
}

impl CompatibilityUsageMeasurement {
    fn validate(&self, route_id: &str) -> bool {
        self.schema_version == "CompatibilityUsageMeasurement-v1"
            && self.route_id == route_id
            && valid_identifier(&self.route_id)
            && self.metric == "legacy-route-invocations"
            && valid_sha256(&self.evidence_sha256)
            && self.window_start_unix_ms < self.window_end_unix_ms
            && self
                .window_end_unix_ms
                .saturating_sub(self.window_start_unix_ms)
                <= MAX_COMPATIBILITY_MEASUREMENT_WINDOW_MS
            && self.observed_invocations <= MAX_COMPATIBILITY_OBSERVED_INVOCATIONS
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CompatibilityBoundary {
    schema_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    deadline_unix_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    product_version: Option<String>,
}

impl CompatibilityBoundary {
    fn validate(&self, measurement_end_unix_ms: u64) -> bool {
        if self.schema_version != "CompatibilityBoundary-v1" {
            return false;
        }
        match (self.deadline_unix_ms, self.product_version.as_deref()) {
            (Some(deadline), None) => {
                deadline > measurement_end_unix_ms
                    && deadline.saturating_sub(measurement_end_unix_ms)
                        <= MAX_COMPATIBILITY_DEADLINE_HORIZON_MS
            }
            (None, Some(version)) => valid_product_version(version),
            _ => false,
        }
    }

    fn semantic_sha256(&self) -> Result<String, ProductMigrationError> {
        serde_json::to_vec(&("CompatibilityBoundary-v1", self))
            .map(|bytes| digest(&bytes))
            .map_err(|_| {
                ProductMigrationError::new("migration-product-compatibility-boundary-invalid")
            })
    }

    fn is_open_at(&self, observation: &CompatibilityBoundaryObservation) -> bool {
        if !observation.validate() {
            return false;
        }
        match (self.deadline_unix_ms, self.product_version.as_deref()) {
            (Some(deadline), None) => observation.observed_at_unix_ms < deadline,
            (None, Some(boundary_version)) => {
                let Some(current) = parse_product_version(&observation.current_product_version)
                else {
                    return false;
                };
                let Some(boundary) = parse_product_version(boundary_version) else {
                    return false;
                };
                current < boundary
            }
            _ => false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CompatibilityRemovalCondition {
    schema_version: String,
    metric: String,
    operator: String,
    threshold: u64,
    required_consecutive_windows: u16,
}

impl CompatibilityRemovalCondition {
    fn validate(&self) -> bool {
        self.schema_version == "CompatibilityRemovalCondition-v1"
            && self.metric == "legacy-route-invocations"
            && self.operator == "less-than-or-equal"
            && self.threshold <= MAX_COMPATIBILITY_OBSERVED_INVOCATIONS
            && (1..=MAX_COMPATIBILITY_CONSECUTIVE_WINDOWS)
                .contains(&self.required_consecutive_windows)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CompatibilityPrerequisites {
    schema_version: String,
    owner_id: String,
    semantic_target_id: String,
    user_facing_warning: String,
    usage_measurement: CompatibilityUsageMeasurement,
    boundary: CompatibilityBoundary,
    removal_condition: CompatibilityRemovalCondition,
}

impl CompatibilityPrerequisites {
    pub(super) fn validate(&self, route_id: &str, canonical_target_id: &str) -> bool {
        self.schema_version == "CompatibilityPrerequisites-v1"
            && valid_identifier(&self.owner_id)
            && valid_stable_identifier(&self.semantic_target_id)
            && self.semantic_target_id == canonical_target_id
            && !self.user_facing_warning.is_empty()
            && self.user_facing_warning.len() <= MAX_COMPATIBILITY_WARNING_BYTES
            && self.user_facing_warning.is_ascii()
            && !self.user_facing_warning.chars().any(char::is_control)
            && self
                .user_facing_warning
                .to_ascii_lowercase()
                .contains("deprecated")
            && self.usage_measurement.validate(route_id)
            && self
                .boundary
                .validate(self.usage_measurement.window_end_unix_ms)
            && self.removal_condition.validate()
    }

    fn semantic_sha256(&self) -> Result<String, ProductMigrationError> {
        let bytes = serde_json::to_vec(&("CompatibilityPrerequisites-v1", self)).map_err(|_| {
            ProductMigrationError::new("migration-product-compatibility-prerequisites-invalid")
        })?;
        if bytes.len() > MAX_MACHINE_OUTPUT_BYTES {
            return Err(ProductMigrationError::new(
                "migration-product-compatibility-prerequisites-too-large",
            ));
        }
        Ok(digest(&bytes))
    }

    fn boundary(&self) -> &CompatibilityBoundary {
        &self.boundary
    }
}

fn valid_product_version(value: &str) -> bool {
    parse_product_version(value).is_some()
}

fn parse_product_version(value: &str) -> Option<[u32; 3]> {
    if value.is_empty() || value.len() > 64 {
        return None;
    }
    let mut components = value.split('.');
    let mut parsed = [0_u32; 3];
    for slot in &mut parsed {
        let component = components.next()?;
        if component.is_empty()
            || (component.len() > 1 && component.starts_with('0'))
            || !component.bytes().all(|byte| byte.is_ascii_digit())
        {
            return None;
        }
        *slot = component.parse().ok()?;
    }
    if components.next().is_some() {
        return None;
    }
    Some(parsed)
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CompatibilityEffectBoundaryPolicy {
    effect_id: String,
    route_id: String,
    prerequisites_sha256: String,
    boundary: CompatibilityBoundary,
    boundary_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CompatibilityBoundaryBinding {
    schema_version: String,
    initial_observation: CompatibilityBoundaryObservation,
    effect_policies: Vec<CompatibilityEffectBoundaryPolicy>,
    binding_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AuthoritySnapshot {
    stable_id: String,
    relative_path: String,
    digest_sha256: String,
    status: SurfaceStatus,
    active_readers: Vec<String>,
    active_writers: Vec<String>,
    public_routes: Vec<String>,
    generated_outputs: Vec<String>,
    physical_bytes_policy: PhysicalBytesPolicy,
}

impl AuthoritySnapshot {
    pub(super) fn from_surface(surface: &InventorySurface) -> Self {
        Self {
            stable_id: surface.stable_id.clone(),
            relative_path: surface.relative_path.clone(),
            digest_sha256: surface.digest_sha256.clone(),
            status: surface.status,
            active_readers: surface.active_readers.clone(),
            active_writers: surface.active_writers.clone(),
            public_routes: surface.public_routes.clone(),
            generated_outputs: surface.generated_outputs.clone(),
            physical_bytes_policy: PhysicalBytesPolicy::Preserve,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn adopted_postcondition(
        source: &InventorySurface,
        status: SurfaceStatus,
        active_readers: Vec<String>,
        active_writers: Vec<String>,
        public_routes: Vec<String>,
        generated_outputs: Vec<String>,
    ) -> Self {
        Self {
            stable_id: source.stable_id.clone(),
            relative_path: source.relative_path.clone(),
            digest_sha256: source.digest_sha256.clone(),
            status,
            active_readers,
            active_writers,
            public_routes,
            generated_outputs,
            physical_bytes_policy: PhysicalBytesPolicy::Preserve,
        }
    }

    pub(crate) fn stable_id(&self) -> &str {
        &self.stable_id
    }

    pub(crate) fn status(&self) -> SurfaceStatus {
        self.status
    }

    pub(crate) fn active_readers(&self) -> &[String] {
        &self.active_readers
    }

    pub(crate) fn active_writers(&self) -> &[String] {
        &self.active_writers
    }

    pub(crate) fn public_routes(&self) -> &[String] {
        &self.public_routes
    }

    pub(crate) fn generated_outputs(&self) -> &[String] {
        &self.generated_outputs
    }

    pub(crate) fn digest_sha256(&self) -> &str {
        &self.digest_sha256
    }

    pub(super) fn semantic_sha256(&self) -> Result<String, ProductMigrationError> {
        let bytes = serde_json::to_vec(&("MigrationAuthoritySnapshot-v1", self)).map_err(|_| {
            ProductMigrationError::new("migration-product-authority-snapshot-invalid")
        })?;
        if bytes.len() > MAX_MACHINE_OUTPUT_BYTES {
            return Err(ProductMigrationError::new(
                "migration-product-authority-snapshot-too-large",
            ));
        }
        Ok(digest(&bytes))
    }

    pub(super) fn validate(&self) -> bool {
        valid_stable_identifier(&self.stable_id)
            && self.relative_path.is_ascii()
            && safe_relative_path(&self.relative_path)
            && valid_sha256(&self.digest_sha256)
            && self.physical_bytes_policy == PhysicalBytesPolicy::Preserve
            && [
                &self.active_readers,
                &self.active_writers,
                &self.public_routes,
                &self.generated_outputs,
            ]
            .into_iter()
            .all(|values| {
                values.windows(2).all(|pair| pair[0] < pair[1])
                    && values.iter().all(|value| safe_reference(value))
            })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PlannedMigrationEffect {
    effect_id: String,
    semantic_key: String,
    route_id: String,
    canonical_target_id: String,
    disposition: PlanDisposition,
    before: AuthoritySnapshot,
    after: AuthoritySnapshot,
    adopted_transition_sha256: String,
    behavior_execution_sha256: String,
    rollback_execution_sha256: String,
    false_pass_control_sha256: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    compatibility_prerequisites: Option<CompatibilityPrerequisites>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    compatibility_prerequisites_sha256: Option<String>,
    physical_bytes_policy: PhysicalBytesPolicy,
}

impl PlannedMigrationEffect {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn issue(
        semantic_key: String,
        route_id: String,
        canonical_target_id: String,
        disposition: PlanDisposition,
        before: AuthoritySnapshot,
        after: AuthoritySnapshot,
        adopted_transition_sha256: String,
        behavior_execution_sha256: String,
        rollback_execution_sha256: String,
        false_pass_control_sha256: BTreeMap<String, String>,
        compatibility_prerequisites: Option<CompatibilityPrerequisites>,
    ) -> Result<Self, ProductMigrationError> {
        let compatibility_prerequisites_sha256 = compatibility_prerequisites
            .as_ref()
            .map(CompatibilityPrerequisites::semantic_sha256)
            .transpose()?;
        let effect_id = planned_effect_digest(
            &semantic_key,
            &route_id,
            &canonical_target_id,
            disposition,
            &before,
            &after,
            &adopted_transition_sha256,
            &behavior_execution_sha256,
            &rollback_execution_sha256,
            &false_pass_control_sha256,
            &compatibility_prerequisites,
            &compatibility_prerequisites_sha256,
        )?;
        let effect = Self {
            effect_id,
            semantic_key,
            route_id,
            canonical_target_id,
            disposition,
            before,
            after,
            adopted_transition_sha256,
            behavior_execution_sha256,
            rollback_execution_sha256,
            false_pass_control_sha256,
            compatibility_prerequisites,
            compatibility_prerequisites_sha256,
            physical_bytes_policy: PhysicalBytesPolicy::Preserve,
        };
        effect.validate()?;
        Ok(effect)
    }

    pub(crate) fn effect_id(&self) -> &str {
        &self.effect_id
    }

    pub(crate) fn semantic_key(&self) -> &str {
        &self.semantic_key
    }

    pub(crate) fn route_id(&self) -> &str {
        &self.route_id
    }

    pub(crate) fn disposition(&self) -> PlanDisposition {
        self.disposition
    }

    pub(crate) fn before(&self) -> &AuthoritySnapshot {
        &self.before
    }

    pub(crate) fn after(&self) -> &AuthoritySnapshot {
        &self.after
    }

    pub(crate) fn compatibility_prerequisites_sha256(&self) -> Option<&str> {
        self.compatibility_prerequisites_sha256.as_deref()
    }

    pub(super) fn validate(&self) -> Result<(), ProductMigrationError> {
        if !valid_sha256(&self.effect_id)
            || !safe_reference(&self.semantic_key)
            || !valid_identifier(&self.route_id)
            || !valid_stable_identifier(&self.canonical_target_id)
            || self.disposition == PlanDisposition::PendingMigration
            || !self.before.validate()
            || !self.after.validate()
            || self.before.stable_id != self.after.stable_id
            || self.before.relative_path != self.after.relative_path
            || self.before.digest_sha256 != self.after.digest_sha256
            || self.physical_bytes_policy != PhysicalBytesPolicy::Preserve
            || !valid_sha256(&self.adopted_transition_sha256)
            || !valid_sha256(&self.behavior_execution_sha256)
            || !valid_sha256(&self.rollback_execution_sha256)
            || self.false_pass_control_sha256.len() != REQUIRED_FALSE_PASS_CONTROLS.len()
            || REQUIRED_FALSE_PASS_CONTROLS.iter().any(|control| {
                self.false_pass_control_sha256
                    .get(*control)
                    .is_none_or(|value| !valid_sha256(value))
            })
            || self.compatibility_prerequisites_sha256
                != self
                    .compatibility_prerequisites
                    .as_ref()
                    .map(CompatibilityPrerequisites::semantic_sha256)
                    .transpose()?
            || self.effect_id
                != planned_effect_digest(
                    &self.semantic_key,
                    &self.route_id,
                    &self.canonical_target_id,
                    self.disposition,
                    &self.before,
                    &self.after,
                    &self.adopted_transition_sha256,
                    &self.behavior_execution_sha256,
                    &self.rollback_execution_sha256,
                    &self.false_pass_control_sha256,
                    &self.compatibility_prerequisites,
                    &self.compatibility_prerequisites_sha256,
                )?
        {
            return Err(ProductMigrationError::new(
                "migration-product-effect-invalid",
            ));
        }
        if self.disposition == PlanDisposition::RetireAuthority
            && (self.after.status != SurfaceStatus::Retired
                || !self.after.active_readers.is_empty()
                || !self.after.active_writers.is_empty()
                || !self.after.public_routes.is_empty()
                || !self.after.generated_outputs.is_empty()
                || self.compatibility_prerequisites.is_some()
                || self.compatibility_prerequisites_sha256.is_some())
        {
            return Err(ProductMigrationError::new(
                "migration-product-retirement-postcondition-invalid",
            ));
        }
        if self.disposition == PlanDisposition::AdoptCompatibility
            && (self.after.status != SurfaceStatus::ContextOnly
                || !self.after.active_readers.is_empty()
                || !self.after.active_writers.is_empty()
                || self.after.public_routes != [self.route_id.clone()]
                || !self.after.generated_outputs.is_empty()
                || self
                    .compatibility_prerequisites
                    .as_ref()
                    .is_none_or(|prerequisites| {
                        !prerequisites.validate(&self.route_id, &self.canonical_target_id)
                    })
                || self.compatibility_prerequisites_sha256.is_none())
        {
            return Err(ProductMigrationError::new(
                "migration-product-compatibility-postcondition-invalid",
            ));
        }
        Ok(())
    }
}

impl CompatibilityBoundaryBinding {
    fn issue(
        initial_observation: CompatibilityBoundaryObservation,
        effects: &[PlannedMigrationEffect],
    ) -> Result<Self, ProductMigrationError> {
        if !initial_observation.validate() {
            return Err(ProductMigrationError::new(
                "migration-product-compatibility-boundary-observation-invalid",
            ));
        }
        let mut effect_policies = Vec::new();
        for effect in effects
            .iter()
            .filter(|effect| effect.disposition == PlanDisposition::AdoptCompatibility)
        {
            let prerequisites = effect.compatibility_prerequisites.as_ref().ok_or_else(|| {
                ProductMigrationError::new(
                    "migration-product-compatibility-boundary-policy-missing",
                )
            })?;
            let prerequisites_sha256 = effect
                .compatibility_prerequisites_sha256
                .as_ref()
                .ok_or_else(|| {
                    ProductMigrationError::new(
                        "migration-product-compatibility-boundary-policy-missing",
                    )
                })?;
            if !prerequisites.boundary().is_open_at(&initial_observation) {
                return Err(ProductMigrationError::new(
                    "migration-product-compatibility-boundary-crossed",
                ));
            }
            effect_policies.push(CompatibilityEffectBoundaryPolicy {
                effect_id: effect.effect_id.clone(),
                route_id: effect.route_id.clone(),
                prerequisites_sha256: prerequisites_sha256.clone(),
                boundary: prerequisites.boundary().clone(),
                boundary_sha256: prerequisites.boundary().semantic_sha256()?,
            });
        }
        if effect_policies.is_empty() {
            return Err(ProductMigrationError::new(
                "migration-product-compatibility-boundary-policy-missing",
            ));
        }
        effect_policies.sort_by(|left, right| left.effect_id.cmp(&right.effect_id));
        let binding_sha256 =
            compatibility_boundary_binding_digest(&initial_observation, &effect_policies)?;
        let value = Self {
            schema_version: "CompatibilityBoundaryBinding-v1".to_owned(),
            initial_observation,
            effect_policies,
            binding_sha256,
        };
        if !value.validate_against(effects)? {
            return Err(ProductMigrationError::new(
                "migration-product-compatibility-boundary-binding-invalid",
            ));
        }
        Ok(value)
    }

    pub(super) fn binding_sha256(&self) -> &str {
        &self.binding_sha256
    }

    pub(super) fn initial_observation(&self) -> &CompatibilityBoundaryObservation {
        &self.initial_observation
    }

    pub(super) fn observation_matches_source_and_progress(
        &self,
        observation: &CompatibilityBoundaryObservation,
        prior: &CompatibilityBoundaryObservation,
    ) -> bool {
        observation.is_same_source_and_monotonic_after(&self.initial_observation)
            && observation.is_same_source_and_monotonic_after(prior)
    }

    pub(super) fn effect_is_open_at(
        &self,
        effect: &PlannedMigrationEffect,
        observation: &CompatibilityBoundaryObservation,
    ) -> bool {
        self.effect_policies
            .iter()
            .find(|policy| policy.effect_id == effect.effect_id)
            .is_some_and(|policy| {
                policy.route_id == effect.route_id
                    && effect.compatibility_prerequisites_sha256.as_deref()
                        == Some(policy.prerequisites_sha256.as_str())
                    && policy.boundary_sha256
                        == policy.boundary.semantic_sha256().unwrap_or_default()
                    && policy.boundary.is_open_at(observation)
            })
    }

    pub(super) fn all_effects_open_at(
        &self,
        effects: &[PlannedMigrationEffect],
        observation: &CompatibilityBoundaryObservation,
    ) -> bool {
        effects
            .iter()
            .filter(|effect| effect.disposition == PlanDisposition::AdoptCompatibility)
            .all(|effect| self.effect_is_open_at(effect, observation))
    }

    fn validate_against(
        &self,
        effects: &[PlannedMigrationEffect],
    ) -> Result<bool, ProductMigrationError> {
        if self.schema_version != "CompatibilityBoundaryBinding-v1"
            || !self.initial_observation.validate()
            || self.effect_policies.is_empty()
            || self
                .effect_policies
                .windows(2)
                .any(|pair| pair[0].effect_id >= pair[1].effect_id)
            || self.binding_sha256
                != compatibility_boundary_binding_digest(
                    &self.initial_observation,
                    &self.effect_policies,
                )?
        {
            return Ok(false);
        }
        let compatibility_effects = effects
            .iter()
            .filter(|effect| effect.disposition == PlanDisposition::AdoptCompatibility)
            .collect::<Vec<_>>();
        if compatibility_effects.len() != self.effect_policies.len() {
            return Ok(false);
        }
        Ok(compatibility_effects.iter().all(|effect| {
            self.effect_policies
                .iter()
                .find(|policy| policy.effect_id == effect.effect_id)
                .is_some_and(|policy| {
                    valid_sha256(&policy.effect_id)
                        && valid_identifier(&policy.route_id)
                        && valid_sha256(&policy.prerequisites_sha256)
                        && valid_sha256(&policy.boundary_sha256)
                        && policy.route_id == effect.route_id
                        && effect.compatibility_prerequisites_sha256.as_deref()
                            == Some(policy.prerequisites_sha256.as_str())
                        && policy.boundary_sha256
                            == policy.boundary.semantic_sha256().unwrap_or_default()
                        && policy.boundary.is_open_at(&self.initial_observation)
                })
        }))
    }
}

fn compatibility_boundary_binding_digest(
    initial_observation: &CompatibilityBoundaryObservation,
    policies: &[CompatibilityEffectBoundaryPolicy],
) -> Result<String, ProductMigrationError> {
    let bytes = serde_json::to_vec(&(
        "CompatibilityBoundaryBinding-v1",
        initial_observation,
        policies,
    ))
    .map_err(|_| {
        ProductMigrationError::new("migration-product-compatibility-boundary-binding-invalid")
    })?;
    if bytes.len() > MAX_MACHINE_OUTPUT_BYTES {
        return Err(ProductMigrationError::new(
            "migration-product-compatibility-boundary-binding-too-large",
        ));
    }
    Ok(digest(&bytes))
}

#[allow(clippy::too_many_arguments)]
fn planned_effect_digest(
    semantic_key: &str,
    route_id: &str,
    canonical_target_id: &str,
    disposition: PlanDisposition,
    before: &AuthoritySnapshot,
    after: &AuthoritySnapshot,
    adopted_transition_sha256: &str,
    behavior_execution_sha256: &str,
    rollback_execution_sha256: &str,
    false_pass_control_sha256: &BTreeMap<String, String>,
    compatibility_prerequisites: &Option<CompatibilityPrerequisites>,
    compatibility_prerequisites_sha256: &Option<String>,
) -> Result<String, ProductMigrationError> {
    let bytes = serde_json::to_vec(&(
        "MigrationPlannedEffect-v1",
        semantic_key,
        route_id,
        canonical_target_id,
        disposition,
        before,
        after,
        adopted_transition_sha256,
        behavior_execution_sha256,
        rollback_execution_sha256,
        false_pass_control_sha256,
        compatibility_prerequisites,
        compatibility_prerequisites_sha256,
    ))
    .map_err(|_| ProductMigrationError::new("migration-product-effect-invalid"))?;
    if bytes.len() > MAX_MACHINE_OUTPUT_BYTES {
        return Err(ProductMigrationError::new(
            "migration-product-effect-too-large",
        ));
    }
    Ok(digest(&bytes))
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProductPlanItem {
    route_id: String,
    source_id: String,
    canonical_target_id: String,
    disposition: PlanDisposition,
    exact_active_readers: Vec<String>,
    exact_active_writers: Vec<String>,
    exact_public_routes: Vec<String>,
    exact_generated_outputs: Vec<String>,
    effect_id: Option<String>,
    reason: String,
}

impl ProductPlanItem {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn issue(
        route_id: String,
        source: &InventorySurface,
        canonical_target_id: String,
        disposition: PlanDisposition,
        effect_id: Option<String>,
        reason: &str,
    ) -> Self {
        Self {
            route_id,
            source_id: source.stable_id.clone(),
            canonical_target_id,
            disposition,
            exact_active_readers: source.active_readers.clone(),
            exact_active_writers: source.active_writers.clone(),
            exact_public_routes: source.public_routes.clone(),
            exact_generated_outputs: source.generated_outputs.clone(),
            effect_id,
            reason: reason.to_owned(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProductMigrationPlanProjection {
    schema_version: String,
    input_binding: MigrationInputBinding,
    contract_id: String,
    registry_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    compatibility_boundary_binding: Option<CompatibilityBoundaryBinding>,
    item_count: usize,
    effect_count: usize,
    items: Vec<ProductPlanItem>,
    effects: Vec<PlannedMigrationEffect>,
    plan_sha256: String,
    projection_sha256: String,
}

#[derive(Clone, Eq, PartialEq)]
pub(crate) struct ProductMigrationPlan {
    projection: ProductMigrationPlanProjection,
}

impl fmt::Debug for ProductMigrationPlan {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProductMigrationPlan")
            .field("plan_sha256", &self.projection.plan_sha256)
            .field("item_count", &self.projection.item_count)
            .field("effect_count", &self.projection.effect_count)
            .finish()
    }
}

impl ProductMigrationPlan {
    pub(super) fn issue(
        input_binding: MigrationInputBinding,
        contract_id: String,
        mut items: Vec<ProductPlanItem>,
        mut effects: Vec<PlannedMigrationEffect>,
        compatibility_boundary_observation: Option<CompatibilityBoundaryObservation>,
    ) -> Result<Self, ProductMigrationError> {
        if items.is_empty() || items.len() > MAX_PRODUCT_ITEMS || effects.len() > items.len() {
            return Err(ProductMigrationError::new(
                "migration-product-plan-count-refused",
            ));
        }
        items.sort_by(|left, right| {
            (&left.route_id, &left.source_id).cmp(&(&right.route_id, &right.source_id))
        });
        effects.sort_by(|left, right| left.semantic_key.cmp(&right.semantic_key));
        if items.windows(2).any(|pair| {
            pair[0].route_id == pair[1].route_id && pair[0].source_id == pair[1].source_id
        }) || effects
            .windows(2)
            .any(|pair| pair[0].semantic_key == pair[1].semantic_key)
        {
            return Err(ProductMigrationError::new(
                "migration-product-plan-duplicate",
            ));
        }
        let has_compatibility_effect = effects
            .iter()
            .any(|effect| effect.disposition == PlanDisposition::AdoptCompatibility);
        let compatibility_boundary_binding =
            match (has_compatibility_effect, compatibility_boundary_observation) {
                (true, Some(observation)) => {
                    Some(CompatibilityBoundaryBinding::issue(observation, &effects)?)
                }
                (true, None) => {
                    return Err(ProductMigrationError::new(
                        "migration-product-compatibility-boundary-authority-required",
                    ));
                }
                (false, None) => None,
                (false, Some(_)) => {
                    return Err(ProductMigrationError::new(
                        "migration-product-compatibility-boundary-unexpected",
                    ));
                }
            };
        let registry_sha256 = input_binding.registry_sha256.clone();
        let plan_sha256 = product_plan_digest(
            &input_binding,
            &contract_id,
            &compatibility_boundary_binding,
            &items,
            &effects,
        )?;
        let boundary_binding_sha256 = compatibility_boundary_binding
            .as_ref()
            .map(CompatibilityBoundaryBinding::binding_sha256)
            .unwrap_or("not-applicable");
        let projection_sha256 = digest(
            format!(
                "migration-product-plan-projection-v2|{}|{}|{}|{}|{}|{}",
                plan_sha256,
                input_binding.binding_sha256,
                contract_id,
                boundary_binding_sha256,
                items.len(),
                effects.len(),
            )
            .as_bytes(),
        );
        let projection = ProductMigrationPlanProjection {
            schema_version: "ProductMigrationPlanProjection-v2".to_owned(),
            input_binding,
            contract_id,
            registry_sha256,
            compatibility_boundary_binding,
            item_count: items.len(),
            effect_count: effects.len(),
            items,
            effects,
            plan_sha256,
            projection_sha256,
        };
        let encoded = serde_json::to_vec(&projection)
            .map_err(|_| ProductMigrationError::new("migration-product-plan-output-invalid"))?;
        if encoded.len() > MAX_MACHINE_OUTPUT_BYTES {
            return Err(ProductMigrationError::new(
                "migration-product-plan-output-too-large",
            ));
        }
        Ok(Self { projection })
    }

    pub(crate) fn plan_sha256(&self) -> &str {
        &self.projection.plan_sha256
    }

    pub(crate) fn input_binding(&self) -> &MigrationInputBinding {
        &self.projection.input_binding
    }

    pub(crate) fn effects(&self) -> &[PlannedMigrationEffect] {
        &self.projection.effects
    }

    pub(super) fn compatibility_boundary_binding(&self) -> Option<&CompatibilityBoundaryBinding> {
        self.projection.compatibility_boundary_binding.as_ref()
    }

    pub(crate) fn items(&self) -> &[ProductPlanItem] {
        &self.projection.items
    }

    pub(crate) fn projection(&self) -> ProductMigrationPlanProjection {
        self.projection.clone()
    }

    pub(crate) fn verify_projection(
        &self,
        projection: &ProductMigrationPlanProjection,
    ) -> Result<(), ProductMigrationError> {
        if &self.projection != projection {
            return Err(ProductMigrationError::new(
                "migration-product-plan-projection-substituted",
            ));
        }
        Ok(())
    }

    pub(super) fn validate(&self) -> Result<(), ProductMigrationError> {
        let expected_boundary_binding =
            match self.projection.compatibility_boundary_binding.as_ref() {
                Some(binding) => {
                    if !binding.validate_against(&self.projection.effects)? {
                        return Err(ProductMigrationError::new("migration-product-plan-mutated"));
                    }
                    binding.binding_sha256()
                }
                None => {
                    if self
                        .projection
                        .effects
                        .iter()
                        .any(|effect| effect.disposition == PlanDisposition::AdoptCompatibility)
                    {
                        return Err(ProductMigrationError::new("migration-product-plan-mutated"));
                    }
                    "not-applicable"
                }
            };
        if self.projection.schema_version != "ProductMigrationPlanProjection-v2"
            || !self.projection.input_binding.validate()
            || self.projection.registry_sha256 != self.projection.input_binding.registry_sha256
            || self.projection.item_count != self.projection.items.len()
            || self.projection.effect_count != self.projection.effects.len()
            || self
                .projection
                .effects
                .iter()
                .any(|effect| effect.validate().is_err())
            || self.projection.plan_sha256
                != product_plan_digest(
                    &self.projection.input_binding,
                    &self.projection.contract_id,
                    &self.projection.compatibility_boundary_binding,
                    &self.projection.items,
                    &self.projection.effects,
                )?
            || self.projection.projection_sha256
                != digest(
                    format!(
                        "migration-product-plan-projection-v2|{}|{}|{}|{}|{}|{}",
                        self.projection.plan_sha256,
                        self.projection.input_binding.binding_sha256,
                        self.projection.contract_id,
                        expected_boundary_binding,
                        self.projection.items.len(),
                        self.projection.effects.len(),
                    )
                    .as_bytes(),
                )
        {
            return Err(ProductMigrationError::new("migration-product-plan-mutated"));
        }
        Ok(())
    }
}

fn product_plan_digest(
    binding: &MigrationInputBinding,
    contract_id: &str,
    compatibility_boundary_binding: &Option<CompatibilityBoundaryBinding>,
    items: &[ProductPlanItem],
    effects: &[PlannedMigrationEffect],
) -> Result<String, ProductMigrationError> {
    let rows = serde_json::to_vec(&(compatibility_boundary_binding, items, effects))
        .map_err(|_| ProductMigrationError::new("migration-product-plan-output-invalid"))?;
    if rows.len() > MAX_MACHINE_OUTPUT_BYTES {
        return Err(ProductMigrationError::new(
            "migration-product-plan-output-too-large",
        ));
    }
    Ok(digest(
        format!(
            "migration-product-plan-v2|{}|{}|{}",
            binding.binding_sha256,
            contract_id,
            digest(&rows),
        )
        .as_bytes(),
    ))
}

fn validate_inventory_paths(inventory: &MigrationInventory) -> Result<(), ProductMigrationError> {
    let mut exact_paths = BTreeSet::new();
    let mut casefold_paths = BTreeMap::<String, String>::new();
    for surface in &inventory.surfaces {
        if !surface.relative_path.is_ascii() || !safe_relative_path(&surface.relative_path) {
            return Err(ProductMigrationError::new(
                "migration-product-inventory-unicode-or-path-refused",
            ));
        }
        if surface.file_kind != SurfaceFileKind::Regular || surface.link_count != 1 {
            return Err(ProductMigrationError::new(
                "migration-product-inventory-file-refused",
            ));
        }
        if !exact_paths.insert(surface.relative_path.clone()) {
            return Err(ProductMigrationError::new(
                "migration-product-inventory-path-alias",
            ));
        }
        let folded = surface.relative_path.to_ascii_lowercase();
        if casefold_paths
            .insert(folded, surface.relative_path.clone())
            .is_some()
        {
            return Err(ProductMigrationError::new(
                "migration-product-inventory-path-collision",
            ));
        }
    }
    Ok(())
}

pub(super) fn exact_input_matches(
    binding: &MigrationInputBinding,
    input: &ProductInputSnapshot,
) -> bool {
    binding.validate() && binding == &MigrationInputBinding::issue(input)
}
