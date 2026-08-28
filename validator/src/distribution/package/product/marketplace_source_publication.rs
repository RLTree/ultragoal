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
        let expected = output
            .inspect(MARKETPLACE_SOURCE_ENTRIES, MARKETPLACE_SOURCE_BYTES)
            .map_err(|_| failure(ProductionPackageErrorId::OutputFailed))?
            .as_deref()
            .map(super::materialize::tree_sha256)
            .transpose()
            .map_err(|_| failure(ProductionPackageErrorId::OutputFailed))?;
        match expected {
            Some(expected) if expected == self.plan.source_tree_sha256() => {
                self.observe_marketplace_source(context, catalog, output)
            }
            Some(_) => Err(failure(ProductionPackageErrorId::OutputFailed)),
            None => self
                .begin_marketplace_source_transition(context, catalog, output, None)
                .map(|(observation, _transaction)| observation),
        }
    }

    pub(crate) fn begin_marketplace_source_transition(
        &self,
        context: &LiveContext,
        catalog: &AuthorityCatalog,
        output: &mut ScopedTree,
        expected_prior_sha256: Option<&str>,
    ) -> Result<
        (
            MarketplaceSourceObservation,
            super::materialize::MaterializeTransaction,
        ),
        ProductionPackageError,
    > {
        verify_product_package(self, context, catalog)?;
        if !supported_marketplace_source_path(output.relative_path()) {
            return Err(failure(ProductionPackageErrorId::OutputFailed));
        }
        let guard = PackageCapture::begin(context)
            .map_err(|_| failure(ProductionPackageErrorId::SourceUnavailable))?;
        let transaction = match expected_prior_sha256 {
            None => {
                super::materialize::materialize_package(&self.plan, &ExpectedTree::Absent, output)
            }
            Some(sha256) => {
                super::materialize::replace_materialized_package(&self.plan, sha256, output)
            }
        }
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
        let observation = MarketplaceSourceObservation {
            context_id: self.context_id.clone(),
            candidate_id: self.candidate_id.clone(),
            catalog_id: self.catalog_id.clone(),
            root_id: output.root_id().into(),
            relative_path: output.relative_path().into(),
            tree_sha256: super::materialize::tree_sha256(&observed)
                .map_err(|_| failure(ProductionPackageErrorId::OutputFailed))?,
        };
        Ok((observation, transaction))
    }

    pub(crate) fn rollback_marketplace_source_transition(
        &self,
        transaction: super::materialize::MaterializeTransaction,
        output: &mut ScopedTree,
        expected_prior_sha256: &str,
    ) -> Result<(), ProductionPackageError> {
        super::materialize::rollback_materialization(transaction, output)
            .map_err(|_| failure(ProductionPackageErrorId::OutputFailed))?;
        let restored = output
            .inspect(MARKETPLACE_SOURCE_ENTRIES, MARKETPLACE_SOURCE_BYTES)
            .map_err(|_| failure(ProductionPackageErrorId::OutputFailed))?
            .ok_or_else(|| failure(ProductionPackageErrorId::OutputFailed))?;
        let restored_sha256 = super::materialize::tree_sha256(&restored)
            .map_err(|_| failure(ProductionPackageErrorId::OutputFailed))?;
        if restored_sha256 != expected_prior_sha256 {
            return Err(failure(ProductionPackageErrorId::OutputFailed));
        }
        Ok(())
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
        if !supported_marketplace_source_path(output.relative_path()) {
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

fn supported_marketplace_source_path(path: &str) -> bool {
    matches!(
        path,
        MARKETPLACE_SOURCE_PATH | super::PERSONAL_MARKETPLACE_SOURCE_RELATIVE
    )
}
