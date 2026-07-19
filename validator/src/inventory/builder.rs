use super::types::{
    ActiveStatus, AuthorityCatalog, AuthorityCatalogDefinition, AuthorityState,
    GeneratedSurfaceIndex, InventoryEntry, InventoryError, catalog_identity_id,
};
use super::{
    ADOPTED_HANDOFF_DIGEST_CONFIG_KEY, context_scopes, discovery, legacy, registry, routing,
    validate,
};
use crate::context::{LiveContext, ReadSession};
use crate::migration::product::ProductMigrationPlanProjection;

pub struct InventoryBuilder<'context> {
    context: &'context LiveContext,
}

impl<'context> InventoryBuilder<'context> {
    pub fn new(context: &'context LiveContext) -> Self {
        Self { context }
    }

    pub fn build(&self) -> Result<AuthorityCatalog, InventoryError> {
        self.build_observed().map(|(catalog, _, _)| catalog)
    }

    pub(crate) fn build_migration_plan(
        &self,
    ) -> Result<ProductMigrationPlanProjection, super::MigrationPlanAdapterError> {
        let (catalog, reads, registry) = self.build_observed()?;
        super::migration_plan::derive(self.context, &catalog, &reads, &registry)
    }

    fn build_observed(
        &self,
    ) -> Result<
        (
            AuthorityCatalog,
            ReadSession,
            super::ObservedMigrationRegistry,
        ),
        InventoryError,
    > {
        self.context
            .revalidate()
            .map_err(|error| InventoryError::Context(error.to_string()))?;
        let reads = self
            .context
            .begin_read_session()
            .map_err(|error| InventoryError::Context(error.to_string()))?;
        let root = reads.root();
        let canonical_root = root.canonicalize().map_err(|error| InventoryError::Io {
            path: root.to_path_buf(),
            message: error.to_string(),
        })?;
        if canonical_root != root {
            return Err(InventoryError::PathEscape(root.to_path_buf()));
        }
        let expected_handoff_digest = self
            .context
            .configuration()
            .public_values
            .get(ADOPTED_HANDOFF_DIGEST_CONFIG_KEY)
            .ok_or_else(|| {
                InventoryError::InvalidRegistry(
                    "root did not bind the adopted handoff manifest digest".to_owned(),
                )
            })?;
        let mut registry = registry::load(&reads, root, expected_handoff_digest)?;
        let activation_sources_current = registry::guard_activation(&reads, root, &mut registry)?;
        let routing = routing::load(&reads, root, &registry.contract_id)?;
        let context_scopes = context_scopes::load(&reads, root, &registry.contract_id)?;
        let discovered = discovery::discover(&reads, root, &registry)?;
        let (mut legacy_entries, legacy_findings) = legacy::discover(&reads, root)?;
        context_scopes.remove_verified_legacy(&mut legacy_entries);
        let context_entry = InventoryEntry {
            stable_id: format!("CONTEXT:{}", self.context.context_id()),
            kind: "live-context".to_owned(),
            owner_role: "OWN-AUTHORITY-KERNEL".to_owned(),
            relative_path: format!("@context/{}", self.context.context_id()),
            digest_sha256: self
                .context
                .context_id()
                .trim_start_matches("sha256:")
                .to_owned(),
            unix_mode: None,
            authority_state: AuthorityState::Context,
            active_status: ActiveStatus::Active,
            generator: Some("HCT-CONTEXT".to_owned()),
            input_provenance: Vec::new(),
            references: Vec::new(),
        };
        let mut findings = registry.findings;
        findings.extend(context_scopes.findings);
        findings.extend(discovered.findings);
        findings.extend(legacy_findings);
        let routing_entry = routing.registry_entry.clone();
        let groups = vec![
            registry.entries,
            discovered.entries,
            legacy_entries,
            context_scopes.entries,
            vec![routing_entry],
            vec![context_entry],
        ];
        let pending_authority =
            routing.classify_pending_authority(&reads, root, &groups, &mut findings)?;
        let (entries, findings) =
            validate::reconcile(root, groups, findings, &routing, &pending_authority);
        reads
            .revalidate()
            .map_err(|error| InventoryError::Context(error.to_string()))?;
        let generated = GeneratedSurfaceIndex::new(
            entries
                .iter()
                .filter(|entry| entry.kind == "generated-surface")
                .cloned()
                .collect(),
        );
        let catalog_id = catalog_identity_id(
            self.context.context_id(),
            &registry.contract_id,
            &registry.counts,
            &entries,
            &findings,
        )?;
        let catalog = AuthorityCatalog::new(AuthorityCatalogDefinition {
            catalog_id,
            context_id: self.context.context_id().to_owned(),
            contract_id: registry.contract_id,
            source_registry_counts: registry.counts,
            entries,
            findings,
            generated_surfaces: generated,
        });
        registry::revalidate_sources(&reads, root, activation_sources_current)?;
        reads
            .revalidate()
            .map_err(|error| InventoryError::Context(error.to_string()))?;
        let migration_registry = routing.into_registry_observation();
        Ok((catalog, reads, migration_registry))
    }
}
