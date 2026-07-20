const MARKETPLACE_SOURCE_PATH: &str = "plugins/harness-ultragoal";
const MARKETPLACE_SOURCE_ENTRIES: usize = 4096;
const MARKETPLACE_SOURCE_BYTES: usize = 65 * 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct MarketplaceSourceObservation {
    context_id: String,
    candidate_id: String,
    catalog_id: String,
    root_id: String,
    relative_path: String,
    tree_sha256: String,
}

impl MarketplaceSourceObservation {
    pub fn context_id(&self) -> &str {
        &self.context_id
    }

    pub fn candidate_id(&self) -> &str {
        &self.candidate_id
    }

    pub fn catalog_id(&self) -> &str {
        &self.catalog_id
    }

    pub fn root_id(&self) -> &str {
        &self.root_id
    }

    pub fn relative_path(&self) -> &str {
        &self.relative_path
    }

    pub fn tree_sha256(&self) -> &str {
        &self.tree_sha256
    }
}

impl ProductionPackageArtifact {
    pub(crate) fn materialize_marketplace_source(
        &self,
        context: &LiveContext,
        catalog: &AuthorityCatalog,
        output: &mut ScopedTree,
    ) -> Result<MarketplaceSourceObservation, ProductionPackageError> {
        verify_product_package(self, context, catalog)?;
        if output.relative_path() != MARKETPLACE_SOURCE_PATH {
            return Err(failure(ProductionPackageErrorId::OutputFailed));
        }
        let guard = PackageCapture::begin(context)
            .map_err(|_| failure(ProductionPackageErrorId::SourceUnavailable))?;
        let expected = match output
            .inspect(MARKETPLACE_SOURCE_ENTRIES, MARKETPLACE_SOURCE_BYTES)
            .map_err(|_| failure(ProductionPackageErrorId::OutputFailed))?
        {
            None => ExpectedTree::Absent,
            Some(tree) => ExpectedTree::ExactDigest(
                super::materialize::tree_sha256(&tree)
                    .map_err(|_| failure(ProductionPackageErrorId::OutputFailed))?,
            ),
        };
        let transaction = super::materialize::materialize_package(&self.plan, &expected, output)
            .map_err(|_| failure(ProductionPackageErrorId::OutputFailed))?;
        let observed = output
            .inspect(MARKETPLACE_SOURCE_ENTRIES, MARKETPLACE_SOURCE_BYTES)
            .map_err(|_| failure(ProductionPackageErrorId::OutputFailed))?
            .ok_or_else(|| failure(ProductionPackageErrorId::OutputFailed))?;
        let post_effect = super::materialize::reconcile(&self.plan, &observed)
            .map_err(|_| failure(ProductionPackageErrorId::OutputFailed))
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
        Ok(MarketplaceSourceObservation {
            context_id: self.context_id.clone(),
            candidate_id: self.candidate_id.clone(),
            catalog_id: self.catalog_id.clone(),
            root_id: output.root_id().into(),
            relative_path: output.relative_path().into(),
            tree_sha256: super::materialize::tree_sha256(&observed)
                .map_err(|_| failure(ProductionPackageErrorId::OutputFailed))?,
        })
    }

    pub(crate) fn verify_marketplace_source(
        &self,
        context: &LiveContext,
        catalog: &AuthorityCatalog,
        output: &ScopedTree,
    ) -> Result<(), ProductionPackageError> {
        self.observe_marketplace_source(context, catalog, output)
            .map(|_| ())
    }

    pub(crate) fn observe_marketplace_source(
        &self,
        context: &LiveContext,
        catalog: &AuthorityCatalog,
        output: &ScopedTree,
    ) -> Result<MarketplaceSourceObservation, ProductionPackageError> {
        verify_product_package(self, context, catalog)?;
        if output.relative_path() != MARKETPLACE_SOURCE_PATH {
            return Err(failure(ProductionPackageErrorId::OutputFailed));
        }
        let observed = output
            .inspect(MARKETPLACE_SOURCE_ENTRIES, MARKETPLACE_SOURCE_BYTES)
            .map_err(|_| failure(ProductionPackageErrorId::OutputFailed))?
            .ok_or_else(|| failure(ProductionPackageErrorId::OutputFailed))?;
        super::materialize::reconcile(&self.plan, &observed)
            .map_err(|_| failure(ProductionPackageErrorId::OutputFailed))?;
        Ok(MarketplaceSourceObservation {
            context_id: self.context_id.clone(),
            candidate_id: self.candidate_id.clone(),
            catalog_id: self.catalog_id.clone(),
            root_id: output.root_id().into(),
            relative_path: output.relative_path().into(),
            tree_sha256: super::materialize::tree_sha256(&observed)
                .map_err(|_| failure(ProductionPackageErrorId::OutputFailed))?,
        })
    }
}
