#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum IdentitySurface {
    Package,
    Installed,
    Cache,
    Marketplace,
    AppRegistry,
    Discovery,
    Runtime,
}

impl IdentitySurface {
    pub const ALL: [Self; 7] = [
        Self::Package,
        Self::Installed,
        Self::Cache,
        Self::Marketplace,
        Self::AppRegistry,
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

/// An identity row issued from a verified distribution observation.
///
/// Callers cannot mint or deserialize rows for the canonical verifier.
///
/// ```compile_fail,E0624
/// use ultragoal::distribution::{IdentitySurface, PackageIdentity, SurfaceIdentity};
/// fn forge(package: PackageIdentity) {
///     let _ = SurfaceIdentity::new(package, IdentitySurface::Package, String::new(), None);
/// }
/// ```
///
/// ```compile_fail,E0277
/// use ultragoal::distribution::SurfaceIdentity;
/// let _: SurfaceIdentity = serde_json::from_str("{}").unwrap();
/// ```
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SurfaceIdentity {
    package: PackageIdentity,
    surface: IdentitySurface,
    observation_sha256: String,
    observed_tree_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    journey_binding_sha256: Option<String>,
}
