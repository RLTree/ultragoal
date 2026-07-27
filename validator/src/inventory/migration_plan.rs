use super::{ActiveStatus, AuthorityCatalog, InventoryEntry, InventoryError};
use crate::context::{LiveContext, ReadSession};
use crate::migration::product::{
    derive_read_only_product_plan, AdoptedRegistrySnapshot, ProductInputSnapshot,
    ProductMigrationError, ProductMigrationPlanProjection,
};
use crate::migration::{
    InventorySurface, InventorySurfaceObservation, MigrationInventory, SurfaceFileKind,
    SurfaceStatus,
};
use sha2::{Digest, Sha256};

#[derive(Debug)]
pub(crate) struct MigrationPlanAdapterError(String);

/// One read-session-bound observation used by the public migration verifier.
///
/// This is intentionally an in-memory adapter, not a durable migration state or
/// second authority inventory. It binds the read-only product plan to the same
/// catalog that established whether any active authority error remains.
pub(crate) struct MigrationVerification {
    catalog_id: String,
    authority_error_codes: Vec<String>,
    plan: ProductMigrationPlanProjection,
}

impl MigrationVerification {
    pub(crate) fn catalog_id(&self) -> &str {
        &self.catalog_id
    }

    pub(crate) fn authority_error_codes(&self) -> &[String] {
        &self.authority_error_codes
    }

    pub(crate) fn plan(&self) -> &ProductMigrationPlanProjection {
        &self.plan
    }

    pub(crate) fn verified(&self) -> bool {
        verified(
            self.authority_error_codes.len(),
            self.plan.pending_count(),
            self.plan.effect_count(),
        )
    }
}

fn verified(authority_error_count: usize, pending_count: usize, effect_count: usize) -> bool {
    authority_error_count == 0 && pending_count == 0 && effect_count == 0
}

impl From<InventoryError> for MigrationPlanAdapterError {
    fn from(error: InventoryError) -> Self {
        Self(format!("migration-plan-inventory:{error}"))
    }
}

impl From<ProductMigrationError> for MigrationPlanAdapterError {
    fn from(error: ProductMigrationError) -> Self {
        Self(error.code().to_owned())
    }
}

impl std::fmt::Display for MigrationPlanAdapterError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

pub(super) fn derive(
    context: &LiveContext,
    catalog: &AuthorityCatalog,
    reads: &ReadSession,
    registry_observation: &super::ObservedMigrationRegistry,
) -> Result<ProductMigrationPlanProjection, MigrationPlanAdapterError> {
    let registry = AdoptedRegistrySnapshot::observed(
        super::MIGRATION_REGISTRY_PATH,
        SurfaceFileKind::Regular,
        1,
        registry_observation.source_identity_sha256(),
        registry_observation.bytes().to_vec(),
    )?;
    let candidate_id = digest(
        &serde_json::to_vec(context.candidate())
            .map_err(|error| MigrationPlanAdapterError(error.to_string()))?,
    );
    let read_session_id = digest(
        format!(
            "migration-read-session-v1\0{}\0{}\0{}\0{}",
            context.context_id(),
            candidate_id,
            catalog.catalog_id(),
            digest(registry.bytes()),
        )
        .as_bytes(),
    );
    let surfaces = catalog
        .entries()
        .iter()
        .map(surface)
        .collect::<Result<Vec<_>, _>>()?;
    let inventory = MigrationInventory::new(
        context.context_id(),
        candidate_id,
        catalog.catalog_id(),
        read_session_id,
        surfaces,
    )
    .map_err(|error| MigrationPlanAdapterError(error.code().to_owned()))?;
    let input = ProductInputSnapshot::observed(inventory, registry)?;
    let plan = derive_read_only_product_plan(&input)?;
    reads
        .revalidate()
        .map_err(|error| MigrationPlanAdapterError(error.to_string()))?;
    Ok(plan.projection())
}

pub(super) fn verify(
    context: &LiveContext,
    catalog: AuthorityCatalog,
    reads: &ReadSession,
    registry_observation: &super::ObservedMigrationRegistry,
) -> Result<MigrationVerification, MigrationPlanAdapterError> {
    catalog
        .revalidate_identity()
        .map_err(MigrationPlanAdapterError::from)?;
    let authority_error_codes = authority_error_codes(&catalog);
    let catalog_id = catalog.catalog_id().to_owned();
    let plan = derive(context, &catalog, reads, registry_observation)?;
    reads
        .revalidate()
        .map_err(|error| MigrationPlanAdapterError(error.to_string()))?;
    Ok(MigrationVerification {
        catalog_id,
        authority_error_codes,
        plan,
    })
}

fn authority_error_codes(catalog: &AuthorityCatalog) -> Vec<String> {
    catalog
        .findings()
        .iter()
        .filter(|finding| finding.severity == crate::inventory::FindingSeverity::Error)
        .map(|finding| finding.code.clone())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn surface(entry: &InventoryEntry) -> Result<InventorySurface, MigrationPlanAdapterError> {
    Ok(InventorySurface::observed(InventorySurfaceObservation {
        stable_id: entry.stable_id.clone(),
        kind: entry.kind.clone(),
        relative_path: entry.relative_path.clone(),
        digest_sha256: observed_digest(entry)?,
        file_kind: SurfaceFileKind::Semantic,
        link_count: 0,
        status: match entry.active_status {
            ActiveStatus::Active => SurfaceStatus::Active,
            ActiveStatus::Definition => SurfaceStatus::Definition,
            ActiveStatus::Retired => SurfaceStatus::Retired,
            ActiveStatus::ContextOnly => SurfaceStatus::ContextOnly,
            ActiveStatus::Candidate | ActiveStatus::Required | ActiveStatus::Missing => {
                SurfaceStatus::Candidate
            }
        },
        active_readers: Vec::new(),
        active_writers: Vec::new(),
        public_routes: Vec::new(),
        generated_outputs: Vec::new(),
    }))
}

fn observed_digest(entry: &InventoryEntry) -> Result<String, MigrationPlanAdapterError> {
    if entry.digest_sha256.is_empty() {
        let bytes = serde_json::to_vec(&("MigrationSemanticInventorySurface-v1", entry))
            .map_err(|error| MigrationPlanAdapterError(error.to_string()))?;
        Ok(digest(&bytes))
    } else if entry.digest_sha256.starts_with("sha256:") {
        Ok(entry.digest_sha256.clone())
    } else {
        Ok(format!("sha256:{}", entry.digest_sha256))
    }
}

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn authority_errors_and_open_migration_work_cannot_verify() {
        let catalog = AuthorityCatalog::canonical_for_test(
            "context".to_owned(),
            "contract".to_owned(),
            BTreeMap::new(),
            Vec::new(),
            vec![crate::inventory::InventoryFinding::error(
                "parallel_authority",
                Some("LEGACY:duplicate"),
                Some("legacy/duplicate.rs"),
                "duplicate authority remains active".to_owned(),
            )],
        )
        .unwrap();
        assert_eq!(authority_error_codes(&catalog), ["parallel_authority"]);
        assert!(!verified(1, 0, 0));
        assert!(!verified(0, 1, 0));
        assert!(!verified(0, 0, 1));
        assert!(verified(0, 0, 0));
    }
}
