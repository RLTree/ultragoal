use crate::distribution::error::{DistributionError, DistributionErrorId, error};
use crate::distribution::spec::digest;
use crate::plugin_manifest::Version;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum IdentitySurface {
    Marketplace,
    Installed,
    Cache,
    Discovery,
    Runtime,
}

impl IdentitySurface {
    pub const ALL: [Self; 5] = [
        Self::Marketplace,
        Self::Installed,
        Self::Cache,
        Self::Discovery,
        Self::Runtime,
    ];
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourceIdentity {
    context_id: String,
    candidate_id: String,
    plugin_id: String,
    version: String,
    catalog_id: String,
    accepted_inventory_sha256: String,
}

impl SourceIdentity {
    pub fn new(
        context_id: String,
        candidate_id: String,
        plugin_id: String,
        version: String,
        catalog_id: String,
        accepted_inventory_sha256: String,
    ) -> Result<Self, DistributionError> {
        if !digest(&context_id)
            || !digest(&candidate_id)
            || plugin_id != "harness-ultragoal"
            || Version::parse(&version).is_none()
            || !digest(&catalog_id)
            || !digest(&accepted_inventory_sha256)
        {
            return Err(error(DistributionErrorId::InvalidSpec));
        }
        Ok(Self {
            context_id,
            candidate_id,
            plugin_id,
            version,
            catalog_id,
            accepted_inventory_sha256,
        })
    }

    pub fn context_id(&self) -> &str {
        &self.context_id
    }
    pub fn candidate_id(&self) -> &str {
        &self.candidate_id
    }
    pub fn plugin_id(&self) -> &str {
        &self.plugin_id
    }
    pub fn version(&self) -> &str {
        &self.version
    }
    pub fn accepted_inventory_sha256(&self) -> &str {
        &self.accepted_inventory_sha256
    }

    pub fn catalog_id(&self) -> &str {
        &self.catalog_id
    }

    pub(crate) fn validate(&self) -> Result<(), DistributionError> {
        Self::new(
            self.context_id.clone(),
            self.candidate_id.clone(),
            self.plugin_id.clone(),
            self.version.clone(),
            self.catalog_id.clone(),
            self.accepted_inventory_sha256.clone(),
        )
        .map(|_| ())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PackageIdentity {
    source: SourceIdentity,
    tree_sha256: String,
    archive_sha256: String,
}

impl PackageIdentity {
    pub fn new(
        source: SourceIdentity,
        tree_sha256: String,
        archive_sha256: String,
    ) -> Result<Self, DistributionError> {
        source.validate()?;
        if !digest(&tree_sha256) || !digest(&archive_sha256) {
            return Err(error(DistributionErrorId::InvalidSpec));
        }
        Ok(Self {
            source,
            tree_sha256,
            archive_sha256,
        })
    }

    pub fn source(&self) -> &SourceIdentity {
        &self.source
    }
    pub fn tree_sha256(&self) -> &str {
        &self.tree_sha256
    }
    pub fn archive_sha256(&self) -> &str {
        &self.archive_sha256
    }

    pub(crate) fn validate(&self) -> Result<(), DistributionError> {
        self.source.validate()?;
        if !digest(&self.tree_sha256) || !digest(&self.archive_sha256) {
            return Err(error(DistributionErrorId::InvalidSpec));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SurfaceIdentity {
    package: PackageIdentity,
    surface: IdentitySurface,
    observation_sha256: String,
    observed_tree_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    journey_binding_sha256: Option<String>,
}

impl SurfaceIdentity {
    pub fn new(
        package: PackageIdentity,
        surface: IdentitySurface,
        observation_sha256: String,
        observed_tree_sha256: Option<String>,
    ) -> Result<Self, DistributionError> {
        package.validate()?;
        if !digest(&observation_sha256)
            || observed_tree_sha256
                .as_deref()
                .is_some_and(|row| !digest(row))
            || matches!(surface, IdentitySurface::Installed | IdentitySurface::Cache)
                != observed_tree_sha256.is_some()
            || observed_tree_sha256
                .as_deref()
                .is_some_and(|row| row != package.tree_sha256())
        {
            return Err(error(DistributionErrorId::ProvenanceMismatch));
        }
        Ok(Self {
            package,
            surface,
            observation_sha256,
            observed_tree_sha256,
            journey_binding_sha256: None,
        })
    }

    pub fn package(&self) -> &PackageIdentity {
        &self.package
    }
    pub const fn surface(&self) -> IdentitySurface {
        self.surface
    }
    pub fn observation_sha256(&self) -> &str {
        &self.observation_sha256
    }

    pub fn observed_tree_sha256(&self) -> Option<&str> {
        self.observed_tree_sha256.as_deref()
    }

    pub fn bind_journey(
        mut self,
        binding: &crate::distribution::host_capability::JourneyBinding,
    ) -> Result<Self, DistributionError> {
        if &self.package != binding.package() {
            return Err(error(DistributionErrorId::ProvenanceMismatch));
        }
        self.journey_binding_sha256 = Some(binding.binding_sha256().into());
        Ok(self)
    }

    pub fn journey_binding_sha256(&self) -> Option<&str> {
        self.journey_binding_sha256.as_deref()
    }

    pub fn from_verified_marketplace(
        snapshot: &crate::distribution::marketplace::MarketplaceSnapshot,
        binding: &crate::distribution::host_capability::JourneyBinding,
    ) -> Result<Self, DistributionError> {
        let package = binding.package();
        let source = package.source();
        if snapshot.context_id() != source.context_id()
            || snapshot.candidate_id() != source.candidate_id()
            || snapshot.plugin_id() != source.plugin_id()
            || snapshot.version() != source.version()
            || snapshot.package_sha256() != package.archive_sha256()
            || snapshot.verdict() != crate::distribution::marketplace::MarketplaceVerdict::Verified
        {
            return Err(error(DistributionErrorId::ProvenanceMismatch));
        }
        let observation = snapshot
            .catalog_sha256()
            .ok_or_else(|| error(DistributionErrorId::ProvenanceMismatch))?;
        Self::new(
            package.clone(),
            IdentitySurface::Marketplace,
            observation.into(),
            None,
        )?
        .bind_journey(binding)
    }

    pub fn from_verified_install(
        snapshot: &crate::distribution::install::InstallSnapshot,
        binding: &crate::distribution::host_capability::JourneyBinding,
    ) -> Result<Self, DistributionError> {
        let package = binding.package();
        let source = package.source();
        if snapshot.context_id() != source.context_id()
            || snapshot.candidate_id() != source.candidate_id()
            || snapshot.package_sha256() != package.archive_sha256()
        {
            return Err(error(DistributionErrorId::ProvenanceMismatch));
        }
        let serialized = serde_json::to_vec(snapshot)
            .map_err(|_| error(DistributionErrorId::ProvenanceMismatch))?;
        Self::new(
            package.clone(),
            IdentitySurface::Installed,
            crate::distribution::reader::sha256(&serialized),
            Some(package.tree_sha256().into()),
        )?
        .bind_journey(binding)
    }

    pub fn from_verified_cache(
        snapshot: &crate::distribution::cache::CacheSnapshot,
        binding: &crate::distribution::host_capability::JourneyBinding,
    ) -> Result<Self, DistributionError> {
        let package = binding.package();
        let source = package.source();
        if snapshot.context_id() != source.context_id()
            || snapshot.candidate_id() != source.candidate_id()
            || snapshot.plugin_id() != source.plugin_id()
            || snapshot.version() != source.version()
            || snapshot.marketplace() != binding.marketplace()
            || snapshot.cache_root_id() != binding.home_id()
            || snapshot.package_tree_sha256() != package.tree_sha256()
        {
            return Err(error(DistributionErrorId::ProvenanceMismatch));
        }
        Self::new(
            package.clone(),
            IdentitySurface::Cache,
            snapshot.observation_sha256().into(),
            Some(package.tree_sha256().into()),
        )?
        .bind_journey(binding)
    }

    pub fn from_verified_discovery(
        observation: &crate::distribution::registry_observation::DiscoveryObservation,
        binding: &crate::distribution::host_capability::JourneyBinding,
    ) -> Result<Self, DistributionError> {
        let package = binding.package();
        let source = package.source();
        if observation.context_id() != source.context_id()
            || observation.candidate_id() != source.candidate_id()
            || observation.verdict() != crate::distribution::model::LayerVerdict::Verified
            || observation.discovery_verdict()
                != crate::distribution::registry_observation::DiscoveryVerdict::Visible
            || observation.binding_sha256() != Some(binding.binding_sha256())
        {
            return Err(error(DistributionErrorId::ProvenanceMismatch));
        }
        let observation_sha256 = observation
            .observation_sha256()
            .ok_or_else(|| error(DistributionErrorId::ProvenanceMismatch))?;
        Self::new(
            package.clone(),
            IdentitySurface::Discovery,
            observation_sha256.into(),
            None,
        )?
        .bind_journey(binding)
    }

    fn validate(&self) -> Result<(), DistributionError> {
        self.package.validate()?;
        Self::new(
            self.package.clone(),
            self.surface,
            self.observation_sha256.clone(),
            self.observed_tree_sha256.clone(),
        )?;
        if self
            .journey_binding_sha256
            .as_deref()
            .is_some_and(|row| !digest(row))
        {
            return Err(error(DistributionErrorId::ProvenanceMismatch));
        }
        Ok(())
    }
}

pub fn verify_bound_surface_chain(
    rows: &[SurfaceIdentity],
    binding: &crate::distribution::host_capability::JourneyBinding,
) -> Result<(), DistributionError> {
    verify_surface_chain(rows)?;
    if rows.iter().any(|row| {
        row.package() != binding.package()
            || row.journey_binding_sha256() != Some(binding.binding_sha256())
    }) {
        return Err(error(DistributionErrorId::ProvenanceMismatch));
    }
    Ok(())
}

pub fn verify_surface_chain(rows: &[SurfaceIdentity]) -> Result<(), DistributionError> {
    if rows.len() != IdentitySurface::ALL.len() {
        return Err(error(DistributionErrorId::ProvenanceMismatch));
    }
    let mut seen = BTreeSet::new();
    let first = rows
        .first()
        .ok_or_else(|| error(DistributionErrorId::ProvenanceMismatch))?;
    for (expected, row) in IdentitySurface::ALL.into_iter().zip(rows) {
        row.validate()?;
        if row.surface != expected || !seen.insert(row.surface) || row.package != first.package {
            return Err(error(DistributionErrorId::ProvenanceMismatch));
        }
    }
    Ok(())
}

pub fn reject_stale_version_reuse(
    previous: &PackageIdentity,
    next: &PackageIdentity,
) -> Result<(), DistributionError> {
    previous.validate()?;
    next.validate()?;
    let previous_version = Version::parse(previous.source.version())
        .ok_or_else(|| error(DistributionErrorId::InvalidSpec))?;
    let next_version = Version::parse(next.source.version())
        .ok_or_else(|| error(DistributionErrorId::InvalidSpec))?;
    let precedence = next_version.precedence_cmp(&previous_version);
    if previous.source.plugin_id != next.source.plugin_id
        || precedence == Ordering::Less
        || (precedence == Ordering::Equal
            && (previous.tree_sha256 != next.tree_sha256
                || previous.archive_sha256 != next.archive_sha256))
    {
        return Err(error(DistributionErrorId::InstallConflict));
    }
    Ok(())
}
