use serde::{Deserialize, Serialize};

#[path = "identity/mod.rs"]
mod identity;
pub use identity::{
    IdentitySurface, PackageIdentity, SourceIdentity, SurfaceIdentity, reject_stale_version_reuse,
    verify_bound_surface_chain, verify_surface_chain,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Capability {
    Filesystem,
    ArchiveWriter,
    Install,
    Cache,
    Marketplace,
    AppRegistry,
    PluginsUi,
    Discovery,
    Runtime,
}

impl Capability {
    pub const ALL: [Self; 9] = [
        Self::Filesystem,
        Self::ArchiveWriter,
        Self::Install,
        Self::Cache,
        Self::Marketplace,
        Self::AppRegistry,
        Self::PluginsUi,
        Self::Discovery,
        Self::Runtime,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Filesystem => "filesystem",
            Self::ArchiveWriter => "archive-writer",
            Self::Install => "install",
            Self::Cache => "cache",
            Self::Marketplace => "marketplace",
            Self::AppRegistry => "app-registry",
            Self::PluginsUi => "plugins-ui",
            Self::Discovery => "discovery",
            Self::Runtime => "runtime",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Layer {
    SourcePluginMetadata,
    SourcePackageInput,
    PackageArchive,
    InstalledBytes,
    CacheBytes,
    MarketplaceCatalog,
    AppRegistry,
    AppRegistryUi,
    Discovery,
    Runtime,
}

impl Layer {
    pub const ALL: [Self; 10] = [
        Self::SourcePluginMetadata,
        Self::SourcePackageInput,
        Self::PackageArchive,
        Self::MarketplaceCatalog,
        Self::InstalledBytes,
        Self::CacheBytes,
        Self::AppRegistry,
        Self::AppRegistryUi,
        Self::Discovery,
        Self::Runtime,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SourcePluginMetadata => "source-plugin-metadata",
            Self::SourcePackageInput => "source-package-input",
            Self::PackageArchive => "package-archive",
            Self::InstalledBytes => "installed-bytes",
            Self::CacheBytes => "cache-bytes",
            Self::MarketplaceCatalog => "marketplace-catalog",
            Self::AppRegistry => "app-registry",
            Self::AppRegistryUi => "app-registry-ui",
            Self::Discovery => "discovery",
            Self::Runtime => "runtime",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|layer| layer.as_str() == value)
    }

    pub const fn capability(self) -> Capability {
        match self {
            Self::SourcePluginMetadata | Self::SourcePackageInput => Capability::Filesystem,
            Self::PackageArchive => Capability::ArchiveWriter,
            Self::InstalledBytes => Capability::Install,
            Self::CacheBytes => Capability::Cache,
            Self::MarketplaceCatalog => Capability::Marketplace,
            Self::AppRegistry => Capability::AppRegistry,
            Self::AppRegistryUi => Capability::PluginsUi,
            Self::Discovery => Capability::Discovery,
            Self::Runtime => Capability::Runtime,
        }
    }

    pub const fn requires_payload(self) -> bool {
        matches!(
            self,
            Self::SourcePackageInput | Self::InstalledBytes | Self::CacheBytes
        )
    }

    pub const fn requires_exposure(self) -> bool {
        matches!(
            self,
            Self::MarketplaceCatalog
                | Self::AppRegistry
                | Self::AppRegistryUi
                | Self::Discovery
                | Self::Runtime
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum HostVerdict {
    Supported,
    PlatformUnavailable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum LayerVerdict {
    Verified,
    DefinitionOnly,
    Missing,
    Unavailable,
    StaleIdentity,
    MixedCandidate,
    Contradicted,
    PayloadMismatch,
    ObservedUnjoined,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum JoinVerdict {
    Match,
    Mismatch,
    NotEvaluated,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct LayerReport {
    pub(crate) layer: Layer,
    pub(crate) verdict: LayerVerdict,
}

impl LayerReport {
    pub const fn layer(&self) -> Layer {
        self.layer
    }

    pub const fn verdict(&self) -> LayerVerdict {
        self.verdict
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct JoinReport {
    pub(crate) from: Layer,
    pub(crate) to: Layer,
    pub(crate) verdict: JoinVerdict,
}

impl JoinReport {
    pub const fn verdict(&self) -> JoinVerdict {
        self.verdict
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DistributionReport {
    pub(crate) context_id: String,
    pub(crate) candidate_id: String,
    pub(crate) host_verdict: HostVerdict,
    pub(crate) layers: Vec<LayerReport>,
    pub(crate) joins: Vec<JoinReport>,
    pub(crate) ladder_sha256: String,
}

impl DistributionReport {
    pub fn accepted(&self) -> bool {
        self.host_verdict == HostVerdict::Supported
            && self
                .layers
                .iter()
                .all(|row| row.verdict == LayerVerdict::Verified)
            && self
                .joins
                .iter()
                .all(|row| row.verdict == JoinVerdict::Match)
    }

    pub fn context_id(&self) -> &str {
        &self.context_id
    }

    pub fn candidate_id(&self) -> &str {
        &self.candidate_id
    }

    pub const fn host_verdict(&self) -> HostVerdict {
        self.host_verdict
    }

    pub fn layers(&self) -> &[LayerReport] {
        &self.layers
    }

    pub fn layer(&self, layer: Layer) -> &LayerReport {
        self.layers
            .iter()
            .find(|row| row.layer == layer)
            .expect("complete layer set")
    }

    pub fn joins(&self) -> &[JoinReport] {
        &self.joins
    }

    pub fn ladder_sha256(&self) -> &str {
        &self.ladder_sha256
    }
}
