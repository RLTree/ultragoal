use super::*;

impl DarwinHostTransactionAdapter {
    pub(super) fn execute_inner(
        &self,
        plan: &DarwinHostTransactionPlan,
        mut hook: impl FnMut(DarwinTestPoint) -> DarwinTestControl,
    ) -> Result<DarwinHostTransactionReport, DarwinHostError> {
        self.require_plan(plan)?;
        let lineage = self.read_lineage()?;
        let anchor = self.read_lineage_anchor()?;
        if let Some(journal) = self.read_journal()? {
            self.journal_authority(&journal, lineage.as_ref(), anchor.as_ref())?;
            return Err(DarwinHostError::new(
                if journal.record.plan_sha256 == plan.plan_sha256 {
                    DarwinHostErrorId::RecoveryRequired
                } else {
                    DarwinHostErrorId::ConcurrentTransaction
                },
            ));
        }
        self.validate_lineage_anchor(lineage.as_ref(), anchor.as_ref())?;
        let current = self.read_all_surfaces()?;
        if self.is_converged(plan, &current) {
            self.validate_idle_execution_authority(
                plan,
                lineage.as_ref(),
                anchor.as_ref(),
                &current,
            )?;
            return Ok(DarwinHostTransactionReport::new(
                DarwinHostTransactionDisposition::AlreadyConverged,
                plan.plan_sha256.clone(),
                self.query(plan)?,
            ));
        }
        self.validate_preconditions(plan, &current)?;
        self.validate_idle_execution_authority(plan, lineage.as_ref(), anchor.as_ref(), &current)?;
        interrupt(&mut hook, DarwinTestPoint::BeforeJournalReservation)?;

        // Admission and precondition reads are not a reservation. Re-read the
        // complete authority tuple immediately before creating the journal so
        // a coherent predecessor substitution cannot be admitted from an old
        // observation.
        let lineage = self.read_lineage()?;
        let anchor = self.read_lineage_anchor()?;
        if let Some(journal) = self.read_journal()? {
            self.journal_authority(&journal, lineage.as_ref(), anchor.as_ref())?;
            return Err(DarwinHostError::new(
                if journal.record.plan_sha256 == plan.plan_sha256 {
                    DarwinHostErrorId::RecoveryRequired
                } else {
                    DarwinHostErrorId::ConcurrentTransaction
                },
            ));
        }
        self.validate_lineage_anchor(lineage.as_ref(), anchor.as_ref())?;
        let current = self.read_all_surfaces()?;
        if self.is_converged(plan, &current) {
            self.validate_idle_execution_authority(
                plan,
                lineage.as_ref(),
                anchor.as_ref(),
                &current,
            )?;
            return Ok(DarwinHostTransactionReport::new(
                DarwinHostTransactionDisposition::AlreadyConverged,
                plan.plan_sha256.clone(),
                self.query(plan)?,
            ));
        }
        self.validate_preconditions(plan, &current)?;
        self.validate_idle_execution_authority(plan, lineage.as_ref(), anchor.as_ref(), &current)?;
        let generation = lineage
            .as_ref()
            .map(|row| row.record.generation)
            .unwrap_or(0)
            .checked_add(1)
            .ok_or_else(|| DarwinHostError::new(DarwinHostErrorId::LineageConflict))?;
        let journal = TransactionJournal {
            schema: "harness-ultragoal.darwin-host-transaction-journal.v1".into(),
            root_id: self.root_id.clone(),
            generation,
            lineage_before_sha256: lineage.as_ref().map(|row| row.tree_sha256.clone()),
            plan_sha256: plan.plan_sha256.clone(),
            operation: plan.operation,
            target: plan.target.clone(),
            marketplace: plan.marketplace.clone(),
            next_step: 0,
            phase: JournalPhase::Pending,
            before: plan
                .steps
                .iter()
                .map(|step| JournalBefore {
                    surface: step.surface,
                    tree_sha256: current[step.surface as usize]
                        .as_ref()
                        .map(|row| row.tree_sha256.clone()),
                })
                .collect(),
        };
        let snapshot = self.create_journal(journal)?;
        interrupt(&mut hook, DarwinTestPoint::AfterJournalReservation)?;
        if let Err(error) = self.validate_reserved_execution_authority(plan, &snapshot, &current) {
            // No host effect has been claimed. Remove only the exact journal
            // this execution reserved; a concurrent claim wins the CAS and is
            // left intact for recovery.
            self.remove_journal(&snapshot)?;
            return Err(error);
        }
        self.run_journal(
            plan,
            snapshot,
            DarwinHostTransactionDisposition::Applied,
            &mut hook,
        )
    }

    pub(super) fn run_journal(
        &self,
        plan: &DarwinHostTransactionPlan,
        mut journal: JournalSnapshot,
        disposition: DarwinHostTransactionDisposition,
        mut hook: impl FnMut(DarwinTestPoint) -> DarwinTestControl,
    ) -> Result<DarwinHostTransactionReport, DarwinHostError> {
        self.require_journal(plan, &journal.record)?;
        let lineage = self.read_lineage()?;
        let anchor = self.read_lineage_anchor()?;
        if self.journal_authority(&journal, lineage.as_ref(), anchor.as_ref())?
            != JournalAuthority::Active
        {
            return Err(DarwinHostError::new(DarwinHostErrorId::JournalConflict));
        }
        self.validate_journal_predecessor_binding(plan, &journal.record, lineage.as_ref())?;
        self.validate_recovery_state(plan, &journal.record)?;
        self.validate_unaffected_surfaces(plan)?;
        while journal.record.next_step < plan.steps.len() {
            let index = journal.record.next_step;
            let step = &plan.steps[index];
            interrupt(&mut hook, DarwinTestPoint::BeforeSurface(step.surface))?;
            // Recheck the stepped surface after this mutation window, while
            // repair-cache also rechecks all six surfaces it intentionally
            // does not rewrite. apply_step performs the final authority read
            // after its own last mutation hook, immediately before the CAS.
            self.validate_recovery_state(plan, &journal.record)?;
            self.validate_unaffected_surfaces(plan)?;
            journal = match journal.record.phase {
                JournalPhase::Pending => self.claim_step(journal, index)?,
                JournalPhase::Applying {
                    step: claimed_step, ..
                } if claimed_step == index => self.reclaim_step(journal, index)?,
                JournalPhase::Applying { .. } => {
                    return Err(DarwinHostError::new(DarwinHostErrorId::JournalCorrupt));
                }
                JournalPhase::Cancelling { .. } => {
                    return Err(DarwinHostError::new(
                        DarwinHostErrorId::ConcurrentTransaction,
                    ));
                }
            };
            self.apply_step(
                plan,
                &journal,
                step,
                journal.record.before[index].tree_sha256.as_deref(),
                &mut hook,
            )?;
            interrupt(&mut hook, DarwinTestPoint::AfterSurface(step.surface))?;
            journal = self.finish_step(journal, index)?;
            interrupt(&mut hook, DarwinTestPoint::AfterProgress(step.surface))?;
        }
        if journal.record.phase != JournalPhase::Pending {
            return Err(DarwinHostError::new(DarwinHostErrorId::JournalCorrupt));
        }
        let current = self.read_all_surfaces()?;
        self.validate_completed_state(plan, &current)?;
        self.commit_lineage(plan, &journal, LineageOutcome::Completed, &mut hook)?;
        interrupt(&mut hook, DarwinTestPoint::BeforeJournalRemoval)?;
        self.remove_journal(&journal)?;
        let snapshot = self.query(plan)?;
        if !self.postcondition(plan, &snapshot) {
            return Err(DarwinHostError::new(DarwinHostErrorId::ObservationChanged));
        }
        Ok(DarwinHostTransactionReport::new(
            disposition,
            plan.plan_sha256.clone(),
            snapshot,
        ))
    }
}
