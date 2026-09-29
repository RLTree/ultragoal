use super::archive;
use super::plan::{PackageEntry, PackagePlan};
use super::spec::PACKAGE_LIMIT;
use crate::distribution::error::{DistributionError, DistributionErrorId, error};
use crate::distribution::model::{PackageIdentity, SourceIdentity};
use crate::distribution::reader::sha256;

#[derive(Clone, Eq, PartialEq)]
pub struct PackageSnapshot {
    context_id: String,
    candidate_id: String,
    package_sha256: String,
    catalog_id: String,
    accepted_inventory_sha256: String,
    source_tree_sha256: String,
    inventory_sha256: String,
    inventory: Vec<u8>,
    archive: Vec<u8>,
    entries: Vec<PackageEntry>,
    identity: PackageIdentity,
}

impl std::fmt::Debug for PackageSnapshot {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PackageSnapshot")
            .field("context_id", &self.context_id)
            .field("candidate_id", &self.candidate_id)
            .field("package_sha256", &self.package_sha256)
            .field("catalog_id", &self.catalog_id)
            .field("accepted_inventory_sha256", &self.accepted_inventory_sha256)
            .field("source_tree_sha256", &self.source_tree_sha256)
            .field("inventory_sha256", &self.inventory_sha256)
            .field("inventory_byte_length", &self.inventory.len())
            .field("archive_byte_length", &self.archive.len())
            .field("entries", &self.entries)
            .field("identity", &self.identity)
            .finish()
    }
}

impl PackageSnapshot {
    pub fn context_id(&self) -> &str {
        &self.context_id
    }
    pub fn candidate_id(&self) -> &str {
        &self.candidate_id
    }
    pub fn package_sha256(&self) -> &str {
        &self.package_sha256
    }
    pub fn catalog_id(&self) -> &str {
        &self.catalog_id
    }
    pub fn accepted_inventory_sha256(&self) -> &str {
        &self.accepted_inventory_sha256
    }
    pub fn source_tree_sha256(&self) -> &str {
        &self.source_tree_sha256
    }
    pub fn inventory_sha256(&self) -> &str {
        &self.inventory_sha256
    }
    pub fn inventory(&self) -> &[u8] {
        &self.inventory
    }
    pub fn archive(&self) -> &[u8] {
        &self.archive
    }
    pub fn entries(&self) -> &[PackageEntry] {
        &self.entries
    }
    pub fn identity(&self) -> &PackageIdentity {
        &self.identity
    }
}

pub trait PackageEffects {
    fn read_package(
        &mut self,
        maximum: usize,
    ) -> Result<Option<Vec<u8>>, crate::distribution::EffectFailure>;
    fn compare_exchange_package(
        &mut self,
        expected_sha256: Option<&str>,
        replacement: Option<&[u8]>,
    ) -> Result<bool, crate::distribution::EffectFailure>;
}

pub fn build_package(
    plan: &PackagePlan,
    effects: &mut impl PackageEffects,
) -> Result<PackageSnapshot, DistributionError> {
    let archive = archive::encode(plan)?;
    let archive_sha256 = sha256(&archive);
    let previous = read(effects)?;
    let expected_prior = previous.as_deref().map(sha256);
    match effects.compare_exchange_package(expected_prior.as_deref(), Some(&archive)) {
        Ok(true) => {}
        Ok(false) => return Err(error(DistributionErrorId::InstallConflict)),
        Err(_) => return Err(error(DistributionErrorId::EffectFailed)),
    }
    let result = match read(effects) {
        Ok(Some(bytes)) => verify_package(plan, &bytes),
        Ok(None) => Err(error(DistributionErrorId::ArchiveMismatch)),
        Err(failure) => Err(failure),
    };
    match result {
        Ok(snapshot) => Ok(snapshot),
        Err(failure) => {
            restore(effects, &archive_sha256, previous.as_deref())?;
            Err(failure)
        }
    }
}

pub fn verify_package(
    plan: &PackagePlan,
    archive_bytes: &[u8],
) -> Result<PackageSnapshot, DistributionError> {
    if archive_bytes.len() > PACKAGE_LIMIT + 1024 * 1024 {
        return Err(error(DistributionErrorId::ObjectTooLarge));
    }
    let decoded = archive::decode(archive_bytes)?;
    archive::verify_plan(&decoded, plan)?;
    let (inventory, inventory_sha256) = archive::inventory(&decoded)?;
    let package_sha256 = sha256(archive_bytes);
    let identity = PackageIdentity::new(
        SourceIdentity::new(
            plan.context_id.clone(),
            plan.candidate_id.clone(),
            plan.plugin_id.clone(),
            plan.version.clone(),
            plan.catalog_id.clone(),
            plan.accepted_inventory_sha256.clone(),
        )?,
        plan.source_tree_sha256.clone(),
        package_sha256.clone(),
    )?;
    Ok(PackageSnapshot {
        context_id: plan.context_id.clone(),
        candidate_id: plan.candidate_id.clone(),
        package_sha256,
        catalog_id: plan.catalog_id.clone(),
        accepted_inventory_sha256: plan.accepted_inventory_sha256.clone(),
        source_tree_sha256: plan.source_tree_sha256.clone(),
        inventory_sha256,
        inventory,
        archive: archive_bytes.to_vec(),
        entries: decoded.entries,
        identity,
    })
}

fn read(effects: &mut impl PackageEffects) -> Result<Option<Vec<u8>>, DistributionError> {
    let bytes = effects
        .read_package(PACKAGE_LIMIT + 1024 * 1024)
        .map_err(|_| error(DistributionErrorId::EffectFailed))?;
    if bytes
        .as_ref()
        .is_some_and(|row| row.len() > PACKAGE_LIMIT + 1024 * 1024)
    {
        return Err(error(DistributionErrorId::ObjectTooLarge));
    }
    Ok(bytes)
}

fn restore(
    effects: &mut impl PackageEffects,
    candidate_sha256: &str,
    previous: Option<&[u8]>,
) -> Result<(), DistributionError> {
    match effects.compare_exchange_package(Some(candidate_sha256), previous) {
        Ok(true) => {}
        Ok(false) => return Err(error(DistributionErrorId::InstallConflict)),
        Err(_) => return Err(error(DistributionErrorId::RollbackFailed)),
    }
    if read(effects)
        .map_err(|_| error(DistributionErrorId::RollbackFailed))?
        .as_deref()
        != previous
    {
        return Err(error(DistributionErrorId::RollbackFailed));
    }
    Ok(())
}
