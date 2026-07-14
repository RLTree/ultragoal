use super::*;

impl DarwinHostTransactionAdapter {
    pub(super) fn apply_step(
        &self,
        plan: &DarwinHostTransactionPlan,
        journal: &JournalSnapshot,
        step: &SurfaceStep,
        before: Option<&str>,
        hook: &mut impl FnMut(DarwinTestPoint) -> DarwinTestControl,
    ) -> Result<(), DarwinHostError> {
        let current = self.read_surface(step.surface)?;
        let actual = current.as_ref().map(|row| row.tree_sha256.as_str());
        if actual == step.desired_tree_sha256.as_deref() {
            return Ok(());
        }
        if actual != before {
            return Err(DarwinHostError::at(
                DarwinHostErrorId::SurfaceConflict,
                step.surface,
            ));
        }
        let replacement = step
            .desired
            .as_ref()
            .map(|bytes| surface_tree(bytes.clone()));
        interrupt(hook, DarwinTestPoint::BeforeCompareExchange(step.surface))?;
        self.validate_surface_effect_authority(plan, journal)?;
        if !self.compare_exchange_tree(
            step.surface.relative_path(),
            before,
            replacement.as_deref(),
            Some(step.surface),
        )? {
            return Err(DarwinHostError::at(
                DarwinHostErrorId::ObservationChanged,
                step.surface,
            ));
        }
        let after = self.read_surface(step.surface)?;
        if after.as_ref().map(|row| row.tree_sha256.as_str()) != step.desired_tree_sha256.as_deref()
        {
            return Err(DarwinHostError::at(
                DarwinHostErrorId::ObservationChanged,
                step.surface,
            ));
        }
        Ok(())
    }

    pub(super) fn validate_surface_effect_authority(
        &self,
        plan: &DarwinHostTransactionPlan,
        journal: &JournalSnapshot,
    ) -> Result<(), DarwinHostError> {
        let current_journal = self
            .read_journal()?
            .ok_or_else(|| DarwinHostError::new(DarwinHostErrorId::JournalConflict))?;
        if current_journal.tree_sha256 != journal.tree_sha256
            || current_journal.record != journal.record
        {
            return Err(DarwinHostError::new(DarwinHostErrorId::JournalConflict));
        }
        let lineage = self.read_lineage()?;
        let anchor = self.read_lineage_anchor()?;
        if self.journal_authority(&current_journal, lineage.as_ref(), anchor.as_ref())?
            != JournalAuthority::Active
        {
            return Err(DarwinHostError::new(DarwinHostErrorId::LineageConflict));
        }
        self.validate_journal_predecessor_binding(plan, &current_journal.record, lineage.as_ref())
    }

    pub(super) fn validate_preconditions(
        &self,
        plan: &DarwinHostTransactionPlan,
        current: &[Option<RawTree>; 7],
    ) -> Result<(), DarwinHostError> {
        let target = |surface: DarwinHostSurface| {
            plan.desired_by_surface
                .get(&surface)
                .and_then(|bytes| surface_tree_sha256(bytes).ok())
        };
        let prior = |surface: DarwinHostSurface| {
            plan.prior_by_surface
                .get(&surface)
                .and_then(|bytes| surface_tree_sha256(bytes).ok())
        };
        for surface in DarwinHostSurface::ALL {
            let actual = current[surface as usize]
                .as_ref()
                .map(|row| row.tree_sha256.as_str());
            let accepted = match plan.operation {
                DarwinHostOperation::Install => actual.is_none(),
                DarwinHostOperation::Update => actual == prior(surface).as_deref(),
                DarwinHostOperation::Reinstall => actual == target(surface).as_deref(),
                DarwinHostOperation::RepairCache if surface == DarwinHostSurface::Cache => {
                    actual.is_none() || actual == prior(surface).as_deref()
                }
                DarwinHostOperation::RepairCache => actual == target(surface).as_deref(),
                DarwinHostOperation::Uninstall => {
                    actual.is_none() || actual == target(surface).as_deref()
                }
            };
            if !accepted {
                return Err(DarwinHostError::at(
                    DarwinHostErrorId::SurfaceConflict,
                    surface,
                ));
            }
        }
        Ok(())
    }

    pub(super) fn is_converged(
        &self,
        plan: &DarwinHostTransactionPlan,
        current: &[Option<RawTree>; 7],
    ) -> bool {
        DarwinHostSurface::ALL.into_iter().all(|surface| {
            let actual = current[surface as usize]
                .as_ref()
                .map(|row| row.tree_sha256.as_str());
            if plan.operation == DarwinHostOperation::Uninstall {
                actual.is_none()
            } else {
                plan.desired_by_surface
                    .get(&surface)
                    .and_then(|bytes| surface_tree_sha256(bytes).ok())
                    .as_deref()
                    == actual
            }
        })
    }

    pub(super) fn postcondition(
        &self,
        plan: &DarwinHostTransactionPlan,
        snapshot: &DarwinHostSnapshot,
    ) -> bool {
        match plan.operation {
            DarwinHostOperation::Uninstall => DarwinHostSurface::ALL
                .into_iter()
                .all(|surface| snapshot.surface(surface).status() == DarwinSurfaceStatus::Absent),
            _ => DarwinHostSurface::ALL
                .into_iter()
                .all(|surface| snapshot.surface(surface).status() == DarwinSurfaceStatus::Verified),
        }
    }
}
