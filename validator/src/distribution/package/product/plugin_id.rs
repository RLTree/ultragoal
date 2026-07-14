pub(super) const PLUGIN_ID: &str = "harness-ultragoal";
pub(super) const SUPPORTED_VERSION: &str = "0.0.12";
pub(super) const SUPPORTED_MANIFEST_PATH: &str = ".codex-plugin/plugin.json";
pub(super) const CANONICAL_SKILLS: [&str; 8] = [
    "harness-ultragoal",
    "repository-fit",
    "routine-work",
    "diagnose-and-observe",
    "goal-run",
    "product-journey-review",
    "prove",
    "improve-and-maintain",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductionPackageErrorId {
    ContextUnavailable,
    CatalogMismatch,
    SourceUnavailable,
    ManifestMismatch,
    MembershipMismatch,
    ArchiveMismatch,
    InventoryMismatch,
    OutputFailed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProductionPackageError {
    id: ProductionPackageErrorId,
}

impl ProductionPackageError {
    pub const fn id(self) -> ProductionPackageErrorId {
        self.id
    }
}

impl std::fmt::Display for ProductionPackageError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self.id {
            ProductionPackageErrorId::ContextUnavailable => {
                "supported package context is unavailable"
            }
            ProductionPackageErrorId::CatalogMismatch => {
                "supported package authority catalog does not match"
            }
            ProductionPackageErrorId::SourceUnavailable => {
                "supported package source is unavailable"
            }
            ProductionPackageErrorId::ManifestMismatch => {
                "supported package manifests do not match"
            }
            ProductionPackageErrorId::MembershipMismatch => {
                "supported package membership does not match"
            }
            ProductionPackageErrorId::ArchiveMismatch => "supported package archive does not match",
            ProductionPackageErrorId::InventoryMismatch => {
                "supported package inventory does not match"
            }
            ProductionPackageErrorId::OutputFailed => {
                "supported package output could not be published"
            }
        })
    }
}

impl std::error::Error for ProductionPackageError {}

fn failure(id: ProductionPackageErrorId) -> ProductionPackageError {
    ProductionPackageError { id }
}

pub struct ProductionPackageSession {
    context: LiveContext,
    catalog_id: String,
    capture: PackageCapture,
}

impl ProductionPackageSession {
    pub fn begin(
        context: &LiveContext,
        catalog: &AuthorityCatalog,
    ) -> Result<Self, ProductionPackageError> {
        validate_context_catalog(context, catalog)?;
        let capture = PackageCapture::begin(context)
            .map_err(|_| failure(ProductionPackageErrorId::SourceUnavailable))?;
        Ok(Self {
            context: context.clone(),
            catalog_id: catalog.catalog_id().to_owned(),
            capture,
        })
    }

    pub fn finish(self) -> Result<ProductionPackageArtifact, ProductionPackageError> {
        let source = self
            .capture
            .finish()
            .map_err(|_| failure(ProductionPackageErrorId::SourceUnavailable))?;
        self.context
            .revalidate()
            .map_err(|_| failure(ProductionPackageErrorId::ContextUnavailable))?;
        let artifact = build_artifact(&self.context, &self.catalog_id, source.as_ref())?;
        self.context
            .revalidate()
            .map_err(|_| failure(ProductionPackageErrorId::ContextUnavailable))?;
        Ok(artifact)
    }
}

#[derive(Clone, Debug)]
pub struct ProductionPackageArtifact {
    context_id: String,
    candidate_id: String,
    catalog_id: String,
    source_snapshot_id: String,
    source_inventory: Vec<u8>,
    plan: PackagePlan,
    snapshot: PackageSnapshot,
    binding: PackageArtifactBinding,
}

impl ProductionPackageArtifact {
    pub fn context_id(&self) -> &str {
        &self.context_id
    }

    pub fn candidate_id(&self) -> &str {
        &self.candidate_id
    }

    pub fn catalog_id(&self) -> &str {
        &self.catalog_id
    }

    pub fn source_snapshot_id(&self) -> &str {
        &self.source_snapshot_id
    }

    pub fn source_inventory(&self) -> &[u8] {
        &self.source_inventory
    }

    pub fn snapshot(&self) -> &PackageSnapshot {
        &self.snapshot
    }

    pub fn publish(
        &self,
        context: &LiveContext,
        catalog: &AuthorityCatalog,
        expected: &ExpectedTree,
        effects: &mut impl MaterializeEffects,
    ) -> Result<PackageArtifactTransaction, ProductionPackageError> {
        verify_product_package(self, context, catalog)?;
        let guard = PackageCapture::begin(context)
            .map_err(|_| failure(ProductionPackageErrorId::SourceUnavailable))?;
        let transaction =
            publish_package_artifact(&self.snapshot, &self.binding, expected, effects)
                .map_err(|_| failure(ProductionPackageErrorId::OutputFailed))?;
        let post_effect = guard
            .finish()
            .map_err(|_| failure(ProductionPackageErrorId::SourceUnavailable))
            .and_then(|_| verify_product_package(self, context, catalog));
        if let Err(problem) = post_effect {
            rollback_package_artifact(transaction, effects)
                .map_err(|_| failure(ProductionPackageErrorId::OutputFailed))?;
            return Err(problem);
        }
        Ok(transaction)
    }
}

pub fn capture_product_package(
    context: &LiveContext,
    catalog: &AuthorityCatalog,
) -> Result<ProductionPackageArtifact, ProductionPackageError> {
    ProductionPackageSession::begin(context, catalog)?.finish()
}

pub fn verify_product_package(
    artifact: &ProductionPackageArtifact,
    context: &LiveContext,
    catalog: &AuthorityCatalog,
) -> Result<(), ProductionPackageError> {
    validate_context_catalog(context, catalog)?;
    let source = PackageCapture::begin(context)
        .map_err(|_| failure(ProductionPackageErrorId::SourceUnavailable))?
        .finish()
        .map_err(|_| failure(ProductionPackageErrorId::SourceUnavailable))?;
    verify_artifact_against_source(artifact, context, catalog, source.as_ref())
}
