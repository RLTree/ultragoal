use super::super::*;

impl DarwinHostTransactionAdapter {
    pub(in super::super) fn commit_lineage(
        &self,
        plan: &DarwinHostTransactionPlan,
        journal: &JournalSnapshot,
        outcome: LineageOutcome,
        hook: &mut impl FnMut(DarwinTestPoint) -> DarwinTestControl,
    ) -> Result<LineageSnapshot, DarwinHostError> {
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
        let record = self.terminal_lineage_record(plan, journal, outcome)?;
        let replacement = surface_tree(lineage_bytes(&record)?);
        let lineage_exchanged = self
            .compare_exchange_tree(
                LINEAGE_PATH,
                journal.record.lineage_before_sha256.as_deref(),
                Some(&replacement),
                None,
            )
            .map_err(|_| DarwinHostError::new(DarwinHostErrorId::LineageConflict))?;
        if !lineage_exchanged {
            let current = self
                .read_lineage()?
                .ok_or_else(|| DarwinHostError::new(DarwinHostErrorId::LineageConflict))?;
            if current.record == record {
                self.ensure_lineage_anchor(journal, &current, anchor.as_ref())?;
                self.validate_lineage_state(&current)?;
                return Ok(current);
            }
            return Err(DarwinHostError::new(DarwinHostErrorId::LineageConflict));
        }
        let current = self
            .read_lineage()?
            .ok_or_else(|| DarwinHostError::new(DarwinHostErrorId::LineageConflict))?;
        if current.record != record {
            return Err(DarwinHostError::new(DarwinHostErrorId::LineageConflict));
        }
        interrupt(hook, DarwinTestPoint::AfterLineageBeforeAnchor)?;
        self.ensure_lineage_anchor(journal, &current, anchor.as_ref())?;
        self.validate_lineage_state(&current)?;
        Ok(current)
    }

    pub(in super::super) fn ensure_lineage_anchor(
        &self,
        journal: &JournalSnapshot,
        lineage: &LineageSnapshot,
        anchor: Option<&LineageAnchorSnapshot>,
    ) -> Result<LineageAnchorSnapshot, DarwinHostError> {
        if let Some(anchor) = anchor
            && self
                .validate_lineage_anchor(Some(lineage), Some(anchor))
                .is_ok()
        {
            return Ok(anchor.clone());
        }
        self.validate_terminal_anchor_transition(journal, lineage, anchor)?;
        let record = TransactionLineageAnchor {
            schema: "harness-ultragoal.darwin-host-lineage-anchor.v1".into(),
            root_id: self.root_id.clone(),
            generation: lineage.record.generation,
            lineage_sha256: lineage.tree_sha256.clone(),
        };
        let replacement = surface_tree(lineage_anchor_bytes(&record)?);
        let _exchanged = self
            .compare_exchange_tree(
                LINEAGE_ANCHOR_PATH,
                anchor.map(|row| row.tree_sha256.as_str()),
                Some(&replacement),
                None,
            )
            .map_err(|_| DarwinHostError::new(DarwinHostErrorId::LineageConflict))?;
        let current = self
            .read_lineage_anchor()?
            .ok_or_else(|| DarwinHostError::new(DarwinHostErrorId::LineageConflict))?;
        if current.record != record {
            return Err(DarwinHostError::new(DarwinHostErrorId::LineageConflict));
        }
        Ok(current)
    }

    pub(in super::super) fn terminal_lineage_record(
        &self,
        plan: &DarwinHostTransactionPlan,
        journal: &JournalSnapshot,
        outcome: LineageOutcome,
    ) -> Result<TransactionLineage, DarwinHostError> {
        match outcome {
            LineageOutcome::Completed
                if journal.record.phase == JournalPhase::Pending
                    && journal.record.next_step == plan.steps.len() => {}
            LineageOutcome::Cancelled
                if journal.record.next_step == 0
                    && matches!(journal.record.phase, JournalPhase::Cancelling { .. }) => {}
            _ => return Err(DarwinHostError::new(DarwinHostErrorId::JournalConflict)),
        }
        let current = self.read_all_surfaces()?;
        match outcome {
            LineageOutcome::Completed => self.validate_completed_state(plan, &current)?,
            LineageOutcome::Cancelled => {
                self.validate_cancelled_state(plan, &journal.record, &current)?
            }
        }
        let terminal = DarwinHostSurface::ALL
            .into_iter()
            .map(|surface| JournalBefore {
                surface,
                tree_sha256: current[surface as usize]
                    .as_ref()
                    .map(|row| row.tree_sha256.clone()),
            })
            .collect();
        Ok(TransactionLineage {
            schema: "harness-ultragoal.darwin-host-transaction-lineage.v1".into(),
            root_id: self.root_id.clone(),
            generation: journal.record.generation,
            previous_lineage_sha256: journal.record.lineage_before_sha256.clone(),
            plan_sha256: journal.record.plan_sha256.clone(),
            operation: journal.record.operation,
            target: journal.record.target.clone(),
            prior: plan.prior.clone(),
            marketplace: journal.record.marketplace.clone(),
            steps: plan
                .steps
                .iter()
                .map(|step| LineagePlanStep {
                    surface: step.surface,
                    desired_tree_sha256: step.desired_tree_sha256.clone(),
                    prior_tree_sha256: step.prior_tree_sha256.clone(),
                })
                .collect(),
            outcome,
            terminal,
        })
    }

    pub(in super::super) fn complete_terminalized(
        &self,
        plan: &DarwinHostTransactionPlan,
        journal: JournalSnapshot,
        outcome: LineageOutcome,
    ) -> Result<DarwinHostTransactionReport, DarwinHostError> {
        let lineage = self
            .read_lineage()?
            .ok_or_else(|| DarwinHostError::new(DarwinHostErrorId::LineageConflict))?;
        if lineage.record != self.terminal_lineage_record(plan, &journal, outcome)? {
            return Err(DarwinHostError::new(DarwinHostErrorId::LineageConflict));
        }
        let anchor = self.read_lineage_anchor()?;
        self.ensure_lineage_anchor(&journal, &lineage, anchor.as_ref())?;
        self.validate_lineage_state(&lineage)?;
        self.remove_journal(&journal)?;
        let snapshot = self.query(plan)?;
        if outcome == LineageOutcome::Completed && !self.postcondition(plan, &snapshot) {
            return Err(DarwinHostError::new(DarwinHostErrorId::ObservationChanged));
        }
        Ok(DarwinHostTransactionReport::new(
            match outcome {
                LineageOutcome::Completed => DarwinHostTransactionDisposition::Recovered,
                LineageOutcome::Cancelled => DarwinHostTransactionDisposition::Cancelled,
            },
            plan.plan_sha256.clone(),
            snapshot,
        ))
    }
}
