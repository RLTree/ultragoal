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
    pub(super) fn from_family(
        route_id: &str,
        canonical_target_id: &str,
        owner_id: &str,
        user_facing_warning: &str,
        usage_measurement_sha256: &str,
        window_start_unix_ms: u64,
        window_end_unix_ms: u64,
        observed_invocations: u64,
        boundary_product_version: &str,
        required_consecutive_windows: u16,
    ) -> Result<Self, ProductMigrationError> {
        let value = Self {
            schema_version: "CompatibilityPrerequisites-v1".to_owned(),
            owner_id: owner_id.to_owned(),
            semantic_target_id: canonical_target_id.to_owned(),
            user_facing_warning: user_facing_warning.to_owned(),
            usage_measurement: CompatibilityUsageMeasurement {
                schema_version: "CompatibilityUsageMeasurement-v1".to_owned(),
                route_id: route_id.to_owned(),
                metric: "legacy-route-invocations".to_owned(),
                evidence_sha256: usage_measurement_sha256.to_owned(),
                window_start_unix_ms,
                window_end_unix_ms,
                observed_invocations,
            },
            boundary: CompatibilityBoundary {
                schema_version: "CompatibilityBoundary-v1".to_owned(),
                deadline_unix_ms: None,
                product_version: Some(boundary_product_version.to_owned()),
            },
            removal_condition: CompatibilityRemovalCondition {
                schema_version: "CompatibilityRemovalCondition-v1".to_owned(),
                metric: "legacy-route-invocations".to_owned(),
                operator: "less-than-or-equal".to_owned(),
                threshold: 0,
                required_consecutive_windows,
            },
        };
        if value.validate(route_id, canonical_target_id) {
            Ok(value)
        } else {
            Err(ProductMigrationError::new(
                "migration-product-compatibility-prerequisites-invalid",
            ))
        }
    }

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
