use super::super::*;

impl DarwinHostTransactionAdapter {
    pub(in super::super) fn validate_idle_authority(
        &self,
        lineage: Option<&LineageSnapshot>,
        anchor: Option<&LineageAnchorSnapshot>,
    ) -> Result<(), DarwinHostError> {
        self.validate_lineage_anchor(lineage, anchor)
    }

    pub(in super::super) fn validate_idle_execution_authority(
        &self,
        plan: &DarwinHostTransactionPlan,
        lineage: Option<&LineageSnapshot>,
        anchor: Option<&LineageAnchorSnapshot>,
        current: &[Option<RawTree>; 7],
    ) -> Result<(), DarwinHostError> {
        self.validate_lineage_anchor(lineage, anchor)?;
        let Some(lineage) = lineage else {
            return Ok(());
        };

        // A cache repair is the sole operation that may intentionally begin
        // with one surface different from the completed terminal state. The
        // authenticated lineage must still name this exact target, its prior
        // cache terminal must be the target cache, and the other six surfaces
        // remain byte-bound to that lineage. Cancelled and uninstall lineages
        // never take this exception.
        if plan.operation == DarwinHostOperation::RepairCache
            && lineage.record.outcome == LineageOutcome::Completed
            && lineage.record.operation != DarwinHostOperation::Uninstall
            && lineage.record.target == plan.target
        {
            let desired_cache = plan
                .desired_by_surface
                .get(&DarwinHostSurface::Cache)
                .ok_or_else(|| DarwinHostError::new(DarwinHostErrorId::InvalidOperation))?;
            let desired_cache_sha256 = surface_tree_sha256(desired_cache)?;
            let prior_cache_sha256 = plan
                .prior_by_surface
                .get(&DarwinHostSurface::Cache)
                .map(|bytes| surface_tree_sha256(bytes))
                .transpose()?;
            for terminal in &lineage.record.terminal {
                if terminal.surface == DarwinHostSurface::Cache {
                    if terminal.tree_sha256.as_deref() != Some(desired_cache_sha256.as_str()) {
                        return Err(DarwinHostError::at(
                            DarwinHostErrorId::LineageConflict,
                            DarwinHostSurface::Cache,
                        ));
                    }
                    let actual = current[DarwinHostSurface::Cache as usize]
                        .as_ref()
                        .map(|row| row.tree_sha256.as_str());
                    if actual == terminal.tree_sha256.as_deref() {
                        self.validate_terminal_surface(lineage, terminal, current)?;
                    } else if actual.is_some() && actual != prior_cache_sha256.as_deref() {
                        return Err(DarwinHostError::at(
                            DarwinHostErrorId::LineageConflict,
                            DarwinHostSurface::Cache,
                        ));
                    }
                } else {
                    self.validate_terminal_surface(lineage, terminal, current)?;
                }
            }
            return Ok(());
        }

        self.validate_lineage_state_against(lineage, current)
    }

    pub(in super::super) fn validate_lineage_anchor(
        &self,
        lineage: Option<&LineageSnapshot>,
        anchor: Option<&LineageAnchorSnapshot>,
    ) -> Result<(), DarwinHostError> {
        match (lineage, anchor) {
            (None, None) => Ok(()),
            (Some(lineage), Some(anchor))
                if anchor.record.generation == lineage.record.generation
                    && anchor.record.lineage_sha256 == lineage.tree_sha256 =>
            {
                Ok(())
            }
            _ => Err(DarwinHostError::new(DarwinHostErrorId::LineageConflict)),
        }
    }

    pub(in super::super) fn validate_terminal_anchor_transition(
        &self,
        journal: &JournalSnapshot,
        lineage: &LineageSnapshot,
        anchor: Option<&LineageAnchorSnapshot>,
    ) -> Result<(), DarwinHostError> {
        if self.validate_lineage_anchor(Some(lineage), anchor).is_ok() {
            return Ok(());
        }
        let predecessor_valid = match (journal.record.lineage_before_sha256.as_deref(), anchor) {
            (None, None) => lineage.record.generation == 1,
            (Some(previous), Some(anchor)) => {
                anchor.record.generation.checked_add(1) == Some(lineage.record.generation)
                    && anchor.record.lineage_sha256 == previous
            }
            _ => false,
        };
        if predecessor_valid {
            Ok(())
        } else {
            Err(DarwinHostError::new(DarwinHostErrorId::LineageConflict))
        }
    }

    pub(in super::super) fn validate_lineage_state(
        &self,
        lineage: &LineageSnapshot,
    ) -> Result<(), DarwinHostError> {
        let current = self.read_all_surfaces()?;
        self.validate_lineage_state_against(lineage, &current)
    }

    pub(in super::super) fn validate_lineage_state_against(
        &self,
        lineage: &LineageSnapshot,
        current: &[Option<RawTree>; 7],
    ) -> Result<(), DarwinHostError> {
        for terminal in &lineage.record.terminal {
            self.validate_terminal_surface(lineage, terminal, current)?;
        }
        Ok(())
    }

    pub(in super::super) fn validate_terminal_surface(
        &self,
        lineage: &LineageSnapshot,
        terminal: &JournalBefore,
        current: &[Option<RawTree>; 7],
    ) -> Result<(), DarwinHostError> {
        let current = &current[terminal.surface as usize];
        if current.as_ref().map(|row| row.tree_sha256.as_str()) != terminal.tree_sha256.as_deref() {
            return Err(DarwinHostError::at(
                DarwinHostErrorId::LineageConflict,
                terminal.surface,
            ));
        }
        if lineage.record.outcome == LineageOutcome::Cancelled {
            return Ok(());
        }
        if lineage.record.operation == DarwinHostOperation::Uninstall {
            if current.is_none() {
                return Ok(());
            }
            return Err(DarwinHostError::at(
                DarwinHostErrorId::LineageConflict,
                terminal.surface,
            ));
        }
        let raw = current.as_ref().ok_or_else(|| {
            DarwinHostError::at(DarwinHostErrorId::LineageConflict, terminal.surface)
        })?;
        let decoded = decode_surface(terminal.surface, &raw.bytes).map_err(|_| {
            DarwinHostError::at(DarwinHostErrorId::LineageConflict, terminal.surface)
        })?;
        if decoded.root_id != self.root_id
            || decoded.marketplace != SUPPORTED_MARKETPLACE
            || decoded.package != lineage.record.target
        {
            return Err(DarwinHostError::at(
                DarwinHostErrorId::LineageConflict,
                terminal.surface,
            ));
        }
        Ok(())
    }

    pub(in super::super) fn journal_authority(
        &self,
        journal: &JournalSnapshot,
        lineage: Option<&LineageSnapshot>,
        anchor: Option<&LineageAnchorSnapshot>,
    ) -> Result<JournalAuthority, DarwinHostError> {
        let active_generation = lineage
            .map(|row| row.record.generation)
            .unwrap_or(0)
            .checked_add(1)
            .ok_or_else(|| DarwinHostError::new(DarwinHostErrorId::LineageConflict))?;
        let lineage_sha256 = lineage.map(|row| row.tree_sha256.as_str());
        if journal.record.generation == active_generation
            && journal.record.lineage_before_sha256.as_deref() == lineage_sha256
        {
            self.validate_lineage_anchor(lineage, anchor)?;
            return Ok(JournalAuthority::Active);
        }
        if let Some(lineage) = lineage
            && lineage.record.generation == journal.record.generation
            && lineage.record.previous_lineage_sha256 == journal.record.lineage_before_sha256
            && lineage.record.plan_sha256 == journal.record.plan_sha256
            && lineage.record.operation == journal.record.operation
            && lineage.record.target == journal.record.target
            && lineage.record.marketplace == journal.record.marketplace
        {
            self.validate_terminal_anchor_transition(journal, lineage, anchor)?;
            return Ok(JournalAuthority::Terminal(lineage.record.outcome));
        }
        Err(DarwinHostError::new(DarwinHostErrorId::LineageConflict))
    }
}
