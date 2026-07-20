const MARKETPLACE_SOURCE_PATH: &str = "plugins/harness-ultragoal";
const MARKETPLACE_SOURCE_ENTRIES: usize = 4096;
const MARKETPLACE_SOURCE_BYTES: usize = 65 * 1024 * 1024;

#[derive(Debug)]
pub(crate) struct MarketplaceSourcePublication {
    root_id: String,
    tree_sha256: String,
}

impl MarketplaceSourcePublication {
    pub(crate) fn root_id(&self) -> &str {
        &self.root_id
    }

    pub(crate) fn tree_sha256(&self) -> &str {
        &self.tree_sha256
    }
}

impl ProductionPackageArtifact {
    pub(crate) fn materialize_marketplace_source(
        &self,
        context: &LiveContext,
        catalog: &AuthorityCatalog,
        output: &mut ScopedTree,
    ) -> Result<MarketplaceSourcePublication, ProductionPackageError> {
        verify_product_package(self, context, catalog)?;
        if output.relative_path() != MARKETPLACE_SOURCE_PATH {
            return Err(failure(ProductionPackageErrorId::OutputFailed));
        }
        let guard = PackageCapture::begin(context)
            .map_err(|_| failure(ProductionPackageErrorId::SourceUnavailable))?;
        let transaction =
            super::materialize::materialize_package(&self.plan, &ExpectedTree::Absent, output)
                .map_err(|_| failure(ProductionPackageErrorId::OutputFailed))?;
        let observed = output
            .inspect(MARKETPLACE_SOURCE_ENTRIES, MARKETPLACE_SOURCE_BYTES)
            .map_err(|_| failure(ProductionPackageErrorId::OutputFailed))?
            .ok_or_else(|| failure(ProductionPackageErrorId::OutputFailed));
        let post_effect = observed
            .and_then(|tree| {
                super::materialize::reconcile(&self.plan, &tree)
                    .map_err(|_| failure(ProductionPackageErrorId::OutputFailed))
            })
            .and_then(|_| {
                guard
                    .finish()
                    .map_err(|_| failure(ProductionPackageErrorId::SourceUnavailable))
            })
            .and_then(|_| verify_product_package(self, context, catalog));
        if let Err(problem) = post_effect {
            super::materialize::rollback_materialization(transaction, output)
                .map_err(|_| failure(ProductionPackageErrorId::OutputFailed))?;
            return Err(problem);
        }
        Ok(MarketplaceSourcePublication {
            root_id: output.root_id().into(),
            tree_sha256: self.snapshot.identity().tree_sha256().into(),
        })
    }

    pub(crate) fn verify_marketplace_source(
        &self,
        context: &LiveContext,
        catalog: &AuthorityCatalog,
        output: &ScopedTree,
    ) -> Result<(), ProductionPackageError> {
        verify_product_package(self, context, catalog)?;
        if output.relative_path() != MARKETPLACE_SOURCE_PATH {
            return Err(failure(ProductionPackageErrorId::OutputFailed));
        }
        let observed = output
            .inspect(MARKETPLACE_SOURCE_ENTRIES, MARKETPLACE_SOURCE_BYTES)
            .map_err(|_| failure(ProductionPackageErrorId::OutputFailed))?
            .ok_or_else(|| failure(ProductionPackageErrorId::OutputFailed))?;
        super::materialize::reconcile(&self.plan, &observed)
            .map_err(|_| failure(ProductionPackageErrorId::OutputFailed))
    }
}
