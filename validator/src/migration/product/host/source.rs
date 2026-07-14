use super::filesystem::FileIdentity;
use super::{HostContext, HostError, MAX_SOURCE_FILE_BYTES, digest_bytes};
use crate::migration::{MigrationInventory, SurfaceFileKind};
use serde::Deserialize;
use std::sync::Arc;

use super::super::{
    AdoptedRegistrySnapshot, MigrationInputBinding, MigrationInputSource, ProductInputSnapshot,
    ProductMigrationError,
};

const REGISTRY_PATH: &str = "migration/authority-routes.json";
const REGISTRY_SOURCE_DOMAIN: &str = "harness-ultragoal.migration-registry-source.v1";
const MAX_REGISTRY_BYTES: u64 = 2 * 1024 * 1024;

#[derive(Deserialize)]
struct SurfaceAnchor {
    relative_path: String,
    digest_sha256: String,
    file_kind: String,
    link_count: u64,
}

pub(crate) struct DarwinMigrationSource {
    context: Arc<HostContext>,
    inventory: MigrationInventory,
}

impl DarwinMigrationSource {
    pub(super) fn new(
        context: Arc<HostContext>,
        inventory: MigrationInventory,
    ) -> Result<Self, HostError> {
        if inventory.candidate_id() != context.config().candidate_id {
            return Err(HostError::new("migration-host-inventory-candidate-refused"));
        }
        let value = Self { context, inventory };
        value.capture_exact()?;
        Ok(value)
    }

    fn capture_exact(&self) -> Result<ProductInputSnapshot, HostError> {
        self.context.verify_static()?;
        self.verify_inventory_surfaces()?;
        let registry =
            self.context
                .repository()
                .read_regular(REGISTRY_PATH, false, MAX_REGISTRY_BYTES)?;
        let registry_digest = digest_bytes(&registry.bytes);
        let source_identity =
            registry_source_identity(self.context.scope_id(), registry.identity, &registry_digest);
        let snapshot = AdoptedRegistrySnapshot::observed(
            REGISTRY_PATH,
            SurfaceFileKind::Regular,
            registry.identity.links,
            source_identity,
            registry.bytes,
        )
        .map_err(|_| HostError::new("migration-host-registry-refused"))?;
        ProductInputSnapshot::observed(self.inventory.clone(), snapshot)
            .map_err(|_| HostError::new("migration-host-input-snapshot-refused"))
    }

    fn verify_inventory_surfaces(&self) -> Result<(), HostError> {
        if self.inventory.surfaces().is_empty() {
            return Err(HostError::new("migration-host-inventory-empty"));
        }
        for surface in self.inventory.surfaces() {
            let value = serde_json::to_value(surface)
                .map_err(|_| HostError::new("migration-host-inventory-surface-invalid"))?;
            let anchor: SurfaceAnchor = serde_json::from_value(value)
                .map_err(|_| HostError::new("migration-host-inventory-surface-invalid"))?;
            if anchor.file_kind != "regular" || anchor.link_count != 1 {
                return Err(HostError::new("migration-host-inventory-file-refused"));
            }
            let observed = self.context.repository().read_regular(
                &anchor.relative_path,
                false,
                MAX_SOURCE_FILE_BYTES,
            )?;
            if observed.identity.links != 1 || digest_bytes(&observed.bytes) != anchor.digest_sha256
            {
                return Err(HostError::new(
                    "migration-host-inventory-source-substituted",
                ));
            }
        }
        self.context.verify_static()
    }
}

impl MigrationInputSource for DarwinMigrationSource {
    fn capture(&mut self) -> Result<ProductInputSnapshot, ProductMigrationError> {
        self.capture_exact().map_err(HostError::product)
    }

    fn revalidate(
        &mut self,
        binding: &MigrationInputBinding,
        _applied_effect_ids: &[String],
    ) -> Result<(), ProductMigrationError> {
        let current = self.capture_exact().map_err(HostError::product)?;
        if &current.binding() != binding {
            return Err(ProductMigrationError::new(
                "migration-host-input-binding-substituted",
            ));
        }
        Ok(())
    }
}

fn registry_source_identity(
    scope_id: &str,
    identity: FileIdentity,
    registry_digest: &str,
) -> String {
    digest_bytes(
        format!(
            "{REGISTRY_SOURCE_DOMAIN}|{scope_id}|{}|{}|{}|{registry_digest}",
            identity.device, identity.inode, identity.links
        )
        .as_bytes(),
    )
}
