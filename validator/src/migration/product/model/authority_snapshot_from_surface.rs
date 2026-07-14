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

    pub(super) fn adopted_postcondition(definition: AdoptedAuthorityPostcondition<'_>) -> Self {
        let AdoptedAuthorityPostcondition {
            source,
            status,
            active_readers,
            active_writers,
            public_routes,
            generated_outputs,
        } = definition;
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

pub(super) struct AdoptedAuthorityPostcondition<'a> {
    pub(super) source: &'a InventorySurface,
    pub(super) status: SurfaceStatus,
    pub(super) active_readers: Vec<String>,
    pub(super) active_writers: Vec<String>,
    pub(super) public_routes: Vec<String>,
    pub(super) generated_outputs: Vec<String>,
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
