impl SurfaceIdentity {
    pub(crate) fn new(
        package: PackageIdentity,
        surface: IdentitySurface,
        observation_sha256: String,
        observed_tree_sha256: Option<String>,
    ) -> Result<Self, DistributionError> {
        if matches!(
            surface,
            IdentitySurface::Package | IdentitySurface::AppRegistry
        ) {
            return Err(error(DistributionErrorId::ProvenanceMismatch));
        }
        Self::issue(package, surface, observation_sha256, observed_tree_sha256)
    }

    fn issue(
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

    pub(crate) fn bind_journey(
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

    pub fn from_published_package(
        snapshot: &crate::distribution::package::PackageSnapshot,
        publication: &crate::distribution::package::PackageArtifactTransaction,
        binding: &crate::distribution::host_capability::JourneyBinding,
    ) -> Result<Self, DistributionError> {
        if snapshot.identity() != binding.package()
            || publication.package_identity() != snapshot.identity()
            || publication.journey_binding_sha256() != binding.binding_sha256()
            || publication.output_root_id() != binding.home_id()
            || publication.output_relative_path()
                != format!(
                    "repository/packages/{}",
                    binding.package().source().plugin_id()
                )
        {
            return Err(error(DistributionErrorId::ProvenanceMismatch));
        }
        Self::issue(
            snapshot.identity().clone(),
            IdentitySurface::Package,
            publication.output_tree_sha256().into(),
            None,
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
            || !observation.is_confined_file_observation()
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

    pub fn from_verified_app_registry(
        observation: &crate::distribution::registry_observation::AppRegistryObservation,
        binding: &crate::distribution::host_capability::JourneyBinding,
    ) -> Result<Self, DistributionError> {
        let source = binding.package().source();
        if observation.context_id() != source.context_id()
            || observation.candidate_id() != source.candidate_id()
            || observation.binding_sha256() != binding.binding_sha256()
            || !observation.is_confined_file_observation()
            || observation.verdict()
                != crate::distribution::registry_observation::AppRegistryVerdict::Verified
        {
            return Err(error(DistributionErrorId::ProvenanceMismatch));
        }
        let observation_sha256 = observation
            .observation_sha256()
            .ok_or_else(|| error(DistributionErrorId::ProvenanceMismatch))?;
        Self::issue(
            binding.package().clone(),
            IdentitySurface::AppRegistry,
            observation_sha256.into(),
            None,
        )?
        .bind_journey(binding)
    }

    fn validate(&self) -> Result<(), DistributionError> {
        self.package.validate()?;
        Self::issue(
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
