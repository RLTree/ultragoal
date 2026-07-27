impl MigrationInventory {
    pub fn new(
        live_context_id: impl Into<String>,
        candidate_id: impl Into<String>,
        catalog_id: impl Into<String>,
        read_session_id: impl Into<String>,
        mut surfaces: Vec<InventorySurface>,
    ) -> Result<Self, MigrationError> {
        if surfaces.is_empty() || surfaces.len() > MAX_SURFACES {
            return Err(MigrationError::new("migration-surface-count-out-of-bounds"));
        }
        surfaces.sort_by(|left, right| left.stable_id.cmp(&right.stable_id));
        let live_context_id = live_context_id.into();
        let candidate_id = candidate_id.into();
        let catalog_id = catalog_id.into();
        let read_session_id = read_session_id.into();
        let inventory_sha256 = inventory_digest(
            &live_context_id,
            &candidate_id,
            &catalog_id,
            &read_session_id,
            &surfaces,
        );
        let inventory = Self {
            live_context_id,
            candidate_id,
            catalog_id,
            read_session_id,
            surfaces,
            inventory_sha256,
        };
        inventory.validate()?;
        Ok(inventory)
    }

    #[cfg(test)]
    pub fn inventory_sha256(&self) -> &str {
        &self.inventory_sha256
    }

    #[cfg(test)]
    pub fn candidate_id(&self) -> &str {
        &self.candidate_id
    }

    #[cfg(test)]
    pub(crate) fn live_context_id(&self) -> &str {
        &self.live_context_id
    }

    #[cfg(test)]
    pub(crate) fn catalog_id(&self) -> &str {
        &self.catalog_id
    }

    #[cfg(test)]
    pub(crate) fn read_session_id(&self) -> &str {
        &self.read_session_id
    }

    pub fn surfaces(&self) -> &[InventorySurface] {
        &self.surfaces
    }

    fn validate(&self) -> Result<(), MigrationError> {
        if !valid_sha256(&self.live_context_id)
            || !valid_sha256(&self.candidate_id)
            || !valid_sha256(&self.catalog_id)
            || !valid_sha256(&self.read_session_id)
        {
            return Err(MigrationError::new("migration-inventory-binding-invalid"));
        }
        let mut ids = BTreeSet::new();
        for surface in &self.surfaces {
            if !ids.insert(surface.stable_id.as_str()) {
                return Err(MigrationError::new("migration-duplicate-surface"));
            }
            if !surface.findings().is_empty() {
                return Err(MigrationError::new("migration-surface-input-refused"));
            }
        }
        if self.inventory_sha256
            != inventory_digest(
                &self.live_context_id,
                &self.candidate_id,
                &self.catalog_id,
                &self.read_session_id,
                &self.surfaces,
            )
        {
            return Err(MigrationError::new("migration-inventory-mutated"));
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn rotate_session_for_test(&mut self, session: impl Into<String>) {
        self.read_session_id = session.into();
        self.inventory_sha256 = inventory_digest(
            &self.live_context_id,
            &self.candidate_id,
            &self.catalog_id,
            &self.read_session_id,
            &self.surfaces,
        );
    }
}

#[cfg(test)]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CompatibilityRoute {
    route_id: String,
    source_id: String,
    canonical_target_id: String,
    owner_id: String,
    warning: String,
    usage_measurement_sha256: String,
    compatibility_boundary: String,
    removal_condition: String,
    equivalence_sha256: String,
    observed_invocations: u64,
    compatibility_window_complete: bool,
}
