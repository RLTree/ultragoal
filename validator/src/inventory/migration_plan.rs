use super::{ActiveStatus, AuthorityCatalog, InventoryEntry, InventoryError};
use crate::context::{LiveContext, ReadSession};
use crate::migration::product::{
    AdoptedRegistrySnapshot, ProductInputSnapshot, ProductMigrationError,
    ProductMigrationPlanProjection, derive_read_only_product_plan,
};
use crate::migration::{
    InventorySurface, InventorySurfaceObservation, MigrationInventory, SurfaceFileKind,
    SurfaceStatus,
};
use sha2::{Digest, Sha256};

#[derive(Debug)]
pub(crate) struct MigrationPlanAdapterError(String);

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
    let surfaces = catalog.entries().iter().map(surface).collect::<Vec<_>>();
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

fn surface(entry: &InventoryEntry) -> InventorySurface {
    InventorySurface::observed(InventorySurfaceObservation {
        stable_id: entry.stable_id.clone(),
        kind: entry.kind.clone(),
        relative_path: entry.relative_path.clone(),
        digest_sha256: canonical_digest(&entry.digest_sha256),
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
    })
}

fn canonical_digest(value: &str) -> String {
    if value.starts_with("sha256:") {
        value.to_owned()
    } else {
        format!("sha256:{value}")
    }
}

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
