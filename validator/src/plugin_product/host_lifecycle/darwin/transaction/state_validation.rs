use super::*;

impl DarwinHostTransactionAdapter {
    pub(super) fn validate_recovery_state(
        &self,
        plan: &DarwinHostTransactionPlan,
        journal: &TransactionJournal,
    ) -> Result<(), DarwinHostError> {
        for (index, step) in plan.steps.iter().enumerate() {
            let current = self.read_surface(step.surface)?;
            let actual = current.as_ref().map(|row| row.tree_sha256.as_str());
            let desired = step.desired_tree_sha256.as_deref();
            let before = journal.before[index].tree_sha256.as_deref();
            let valid = if index < journal.next_step {
                actual == desired
            } else {
                actual == before || actual == desired
            };
            if !valid {
                return Err(DarwinHostError::at(
                    DarwinHostErrorId::SurfaceConflict,
                    step.surface,
                ));
            }
        }
        Ok(())
    }

    pub(super) fn validate_reserved_execution_authority(
        &self,
        plan: &DarwinHostTransactionPlan,
        journal: &JournalSnapshot,
        reserved: &[Option<RawTree>; 7],
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
        if self.journal_authority(journal, lineage.as_ref(), anchor.as_ref())?
            != JournalAuthority::Active
        {
            return Err(DarwinHostError::new(DarwinHostErrorId::LineageConflict));
        }
        let current = self.read_all_surfaces()?;
        self.validate_idle_execution_authority(plan, lineage.as_ref(), anchor.as_ref(), &current)?;
        self.validate_preconditions(plan, &current)?;
        for surface in DarwinHostSurface::ALL {
            let expected = reserved[surface as usize]
                .as_ref()
                .map(|row| row.tree_sha256.as_str());
            let actual = current[surface as usize]
                .as_ref()
                .map(|row| row.tree_sha256.as_str());
            if actual != expected {
                return Err(DarwinHostError::at(
                    DarwinHostErrorId::ObservationChanged,
                    surface,
                ));
            }
        }
        Ok(())
    }

    pub(super) fn validate_journal_predecessor_binding(
        &self,
        plan: &DarwinHostTransactionPlan,
        journal: &TransactionJournal,
        lineage: Option<&LineageSnapshot>,
    ) -> Result<(), DarwinHostError> {
        let Some(lineage) = lineage else {
            return Ok(());
        };
        let repair_cache_drift = plan.operation == DarwinHostOperation::RepairCache
            && lineage.record.outcome == LineageOutcome::Completed
            && lineage.record.operation != DarwinHostOperation::Uninstall
            && lineage.record.target == plan.target;
        for terminal in &lineage.record.terminal {
            let before = match plan
                .steps
                .iter()
                .position(|step| step.surface == terminal.surface)
            {
                Some(index) => journal.before[index].tree_sha256.clone(),
                None => plan
                    .desired_by_surface
                    .get(&terminal.surface)
                    .map(|bytes| surface_tree_sha256(bytes))
                    .transpose()?,
            };
            if repair_cache_drift && terminal.surface == DarwinHostSurface::Cache {
                let desired = plan
                    .desired_by_surface
                    .get(&DarwinHostSurface::Cache)
                    .ok_or_else(|| DarwinHostError::new(DarwinHostErrorId::InvalidOperation))?;
                let desired_sha256 = surface_tree_sha256(desired)?;
                let prior_sha256 = plan
                    .prior_by_surface
                    .get(&DarwinHostSurface::Cache)
                    .map(|bytes| surface_tree_sha256(bytes))
                    .transpose()?;
                if terminal.tree_sha256.as_deref() != Some(desired_sha256.as_str())
                    || before.is_some() && before != prior_sha256
                {
                    return Err(DarwinHostError::at(
                        DarwinHostErrorId::LineageConflict,
                        terminal.surface,
                    ));
                }
            } else if before != terminal.tree_sha256 {
                return Err(DarwinHostError::at(
                    DarwinHostErrorId::LineageConflict,
                    terminal.surface,
                ));
            }
        }
        Ok(())
    }

    pub(super) fn validate_unaffected_surfaces(
        &self,
        plan: &DarwinHostTransactionPlan,
    ) -> Result<(), DarwinHostError> {
        if plan.operation != DarwinHostOperation::RepairCache {
            return Ok(());
        }
        for surface in DarwinHostSurface::ALL {
            if surface == DarwinHostSurface::Cache {
                continue;
            }
            let current = self.read_surface(surface)?;
            let expected = plan
                .desired_by_surface
                .get(&surface)
                .map(|bytes| surface_tree_sha256(bytes))
                .transpose()?;
            if current.as_ref().map(|row| row.tree_sha256.as_str()) != expected.as_deref() {
                return Err(DarwinHostError::at(
                    DarwinHostErrorId::SurfaceConflict,
                    surface,
                ));
            }
        }
        Ok(())
    }

    pub(super) fn validate_completed_state(
        &self,
        plan: &DarwinHostTransactionPlan,
        current: &[Option<RawTree>; 7],
    ) -> Result<(), DarwinHostError> {
        for surface in DarwinHostSurface::ALL {
            let actual = current[surface as usize]
                .as_ref()
                .map(|row| row.tree_sha256.as_str());
            let expected = if plan.operation == DarwinHostOperation::Uninstall {
                None
            } else {
                plan.desired_by_surface
                    .get(&surface)
                    .map(|bytes| surface_tree_sha256(bytes))
                    .transpose()?
            };
            if actual != expected.as_deref() {
                return Err(DarwinHostError::at(
                    DarwinHostErrorId::ObservationChanged,
                    surface,
                ));
            }
        }
        Ok(())
    }

    pub(super) fn validate_cancelled_state(
        &self,
        plan: &DarwinHostTransactionPlan,
        journal: &TransactionJournal,
        current: &[Option<RawTree>; 7],
    ) -> Result<(), DarwinHostError> {
        for surface in DarwinHostSurface::ALL {
            let actual = current[surface as usize]
                .as_ref()
                .map(|row| row.tree_sha256.as_str());
            let expected = match plan.steps.iter().position(|step| step.surface == surface) {
                Some(index) => journal.before[index].tree_sha256.clone(),
                None => plan
                    .desired_by_surface
                    .get(&surface)
                    .map(|bytes| surface_tree_sha256(bytes))
                    .transpose()?,
            };
            if actual != expected.as_deref() {
                return Err(DarwinHostError::at(
                    DarwinHostErrorId::CancellationUnsafe,
                    surface,
                ));
            }
        }
        Ok(())
    }
}
