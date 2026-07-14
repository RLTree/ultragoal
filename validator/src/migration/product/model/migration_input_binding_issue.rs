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
