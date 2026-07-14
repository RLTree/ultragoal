use super::*;

impl DarwinHostTransactionAdapter {
    pub fn open(root: ConfinedRoot) -> Result<Self, DarwinHostError> {
        if !cfg!(target_os = "macos") {
            return Err(DarwinHostError::new(DarwinHostErrorId::UnsupportedPlatform));
        }
        let root_id = root.root_id().to_owned();
        // A read proves that the retained descriptor still owns the supplied
        // root without creating adapter state.
        ScopedTree::new(root.clone(), JOURNAL_PATH)
            .and_then(|row| row.inspect(TREE_ENTRY_LIMIT, 1024 * 1024))
            .map_err(map_distribution)?;
        let adapter = Self {
            root,
            root_id,
            lineage_floor: Arc::new(Mutex::new(None)),
        };
        let lineage = adapter.read_lineage()?;
        let anchor = adapter.read_lineage_anchor()?;
        match adapter.read_journal()? {
            Some(journal) => {
                if matches!(
                    adapter.journal_authority(&journal, lineage.as_ref(), anchor.as_ref())?,
                    JournalAuthority::Terminal(_)
                ) {
                    adapter.validate_lineage_state(lineage.as_ref().ok_or_else(|| {
                        DarwinHostError::new(DarwinHostErrorId::LineageConflict)
                    })?)?;
                }
            }
            None => adapter.validate_idle_authority(lineage.as_ref(), anchor.as_ref())?,
        }
        Ok(adapter)
    }

    pub fn root_id(&self) -> &str {
        &self.root_id
    }

    pub fn plan_install(
        &self,
        package: &PackageSnapshot,
        marketplace: &str,
    ) -> Result<DarwinHostTransactionPlan, DarwinHostError> {
        DarwinHostTransactionPlan::build(
            &self.root_id,
            DarwinHostOperation::Install,
            package,
            None,
            marketplace,
        )
    }

    pub fn plan_update(
        &self,
        package: &PackageSnapshot,
        previous: &PackageSnapshot,
        marketplace: &str,
    ) -> Result<DarwinHostTransactionPlan, DarwinHostError> {
        DarwinHostTransactionPlan::build(
            &self.root_id,
            DarwinHostOperation::Update,
            package,
            Some(previous),
            marketplace,
        )
    }

    pub fn plan_reinstall(
        &self,
        package: &PackageSnapshot,
        marketplace: &str,
    ) -> Result<DarwinHostTransactionPlan, DarwinHostError> {
        DarwinHostTransactionPlan::build(
            &self.root_id,
            DarwinHostOperation::Reinstall,
            package,
            None,
            marketplace,
        )
    }

    pub fn plan_repair_cache(
        &self,
        package: &PackageSnapshot,
        stale: &PackageSnapshot,
        marketplace: &str,
    ) -> Result<DarwinHostTransactionPlan, DarwinHostError> {
        DarwinHostTransactionPlan::build(
            &self.root_id,
            DarwinHostOperation::RepairCache,
            package,
            Some(stale),
            marketplace,
        )
    }

    pub fn plan_uninstall(
        &self,
        package: &PackageSnapshot,
        marketplace: &str,
    ) -> Result<DarwinHostTransactionPlan, DarwinHostError> {
        DarwinHostTransactionPlan::build(
            &self.root_id,
            DarwinHostOperation::Uninstall,
            package,
            None,
            marketplace,
        )
    }

    pub fn query(
        &self,
        plan: &DarwinHostTransactionPlan,
    ) -> Result<DarwinHostSnapshot, DarwinHostError> {
        self.require_plan(plan)?;
        let lineage = self.read_lineage()?;
        let anchor = self.read_lineage_anchor()?;
        let journal = self.read_journal()?;
        match journal.as_ref() {
            Some(journal) => {
                if matches!(
                    self.journal_authority(journal, lineage.as_ref(), anchor.as_ref())?,
                    JournalAuthority::Terminal(_)
                ) {
                    self.validate_lineage_state(lineage.as_ref().ok_or_else(|| {
                        DarwinHostError::new(DarwinHostErrorId::LineageConflict)
                    })?)?;
                }
            }
            None => self.validate_idle_authority(lineage.as_ref(), anchor.as_ref())?,
        }
        let rows = DarwinHostSurface::ALL
            .map(|surface| self.observe_surface(plan, surface))
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?;
        let recovery_plan_sha256 = match journal {
            Some(journal) => Some(journal.record.plan_sha256),
            None => None,
        };
        DarwinHostSnapshot::assemble(rows, recovery_plan_sha256)
    }

    pub fn execute(
        &self,
        plan: &DarwinHostTransactionPlan,
    ) -> Result<DarwinHostTransactionReport, DarwinHostError> {
        self.execute_inner(plan, |_| DarwinTestControl::Continue)
    }

    #[cfg(test)]
    pub fn execute_with_test_hook(
        &self,
        plan: &DarwinHostTransactionPlan,
        hook: impl FnMut(DarwinTestPoint) -> DarwinTestControl,
    ) -> Result<DarwinHostTransactionReport, DarwinHostError> {
        self.execute_inner(plan, hook)
    }

    pub fn recover(
        &self,
        plan: &DarwinHostTransactionPlan,
    ) -> Result<DarwinHostTransactionReport, DarwinHostError> {
        self.recover_inner(plan, |_| DarwinTestControl::Continue)
    }

    #[cfg(test)]
    pub fn recover_with_test_hook(
        &self,
        plan: &DarwinHostTransactionPlan,
        hook: impl FnMut(DarwinTestPoint) -> DarwinTestControl,
    ) -> Result<DarwinHostTransactionReport, DarwinHostError> {
        self.recover_inner(plan, hook)
    }
}
