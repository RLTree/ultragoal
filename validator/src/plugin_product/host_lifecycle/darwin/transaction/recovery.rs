use super::*;

impl DarwinHostTransactionAdapter {
    pub(super) fn recover_inner(
        &self,
        plan: &DarwinHostTransactionPlan,
        mut hook: impl FnMut(DarwinTestPoint) -> DarwinTestControl,
    ) -> Result<DarwinHostTransactionReport, DarwinHostError> {
        self.require_plan(plan)?;
        let journal = self
            .read_journal()?
            .ok_or_else(|| DarwinHostError::new(DarwinHostErrorId::JournalConflict))?;
        self.require_journal(plan, &journal.record)?;
        let lineage = self.read_lineage()?;
        let anchor = self.read_lineage_anchor()?;
        match self.journal_authority(&journal, lineage.as_ref(), anchor.as_ref())? {
            JournalAuthority::Terminal(outcome) => {
                self.validate_lineage_state(
                    lineage
                        .as_ref()
                        .ok_or_else(|| DarwinHostError::new(DarwinHostErrorId::LineageConflict))?,
                )?;
                return self.complete_terminalized(plan, journal, outcome);
            }
            JournalAuthority::Active => {}
        }
        if matches!(journal.record.phase, JournalPhase::Cancelling { .. }) {
            let journal = self.claim_cancellation(journal)?;
            return self.complete_cancellation(plan, journal, &mut hook);
        }
        self.run_journal(
            plan,
            journal,
            DarwinHostTransactionDisposition::Recovered,
            hook,
        )
    }

    pub fn cancel(
        &self,
        plan: &DarwinHostTransactionPlan,
    ) -> Result<DarwinHostTransactionReport, DarwinHostError> {
        self.cancel_inner(plan, |_| DarwinTestControl::Continue)
    }

    #[cfg(test)]
    pub fn cancel_with_test_hook(
        &self,
        plan: &DarwinHostTransactionPlan,
        hook: impl FnMut(DarwinTestPoint) -> DarwinTestControl,
    ) -> Result<DarwinHostTransactionReport, DarwinHostError> {
        self.cancel_inner(plan, hook)
    }

    pub(super) fn cancel_inner(
        &self,
        plan: &DarwinHostTransactionPlan,
        mut hook: impl FnMut(DarwinTestPoint) -> DarwinTestControl,
    ) -> Result<DarwinHostTransactionReport, DarwinHostError> {
        self.require_plan(plan)?;
        let journal = self
            .read_journal()?
            .ok_or_else(|| DarwinHostError::new(DarwinHostErrorId::JournalConflict))?;
        self.require_journal(plan, &journal.record)?;
        let lineage = self.read_lineage()?;
        let anchor = self.read_lineage_anchor()?;
        if self.journal_authority(&journal, lineage.as_ref(), anchor.as_ref())?
            != JournalAuthority::Active
        {
            return Err(DarwinHostError::new(DarwinHostErrorId::CancellationUnsafe));
        }
        let journal = self.claim_cancellation(journal)?;
        interrupt(&mut hook, DarwinTestPoint::AfterCancellationClaim)?;
        self.complete_cancellation(plan, journal, &mut hook)
    }

    pub(super) fn complete_cancellation(
        &self,
        plan: &DarwinHostTransactionPlan,
        journal: JournalSnapshot,
        hook: &mut impl FnMut(DarwinTestPoint) -> DarwinTestControl,
    ) -> Result<DarwinHostTransactionReport, DarwinHostError> {
        if journal.record.next_step != 0
            || !matches!(journal.record.phase, JournalPhase::Cancelling { .. })
        {
            return Err(DarwinHostError::new(DarwinHostErrorId::CancellationUnsafe));
        }
        let lineage = self.read_lineage()?;
        self.validate_journal_predecessor_binding(plan, &journal.record, lineage.as_ref())?;
        let current = self.read_all_surfaces()?;
        self.validate_cancelled_state(plan, &journal.record, &current)?;
        self.commit_lineage(plan, &journal, LineageOutcome::Cancelled, hook)?;
        interrupt(hook, DarwinTestPoint::BeforeCancellationRemoval)?;
        self.remove_journal(&journal)?;
        let snapshot = self.query(plan)?;
        if snapshot.recovery_plan_sha256().is_some() {
            return Err(DarwinHostError::new(DarwinHostErrorId::ObservationChanged));
        }
        Ok(DarwinHostTransactionReport::new(
            DarwinHostTransactionDisposition::Cancelled,
            plan.plan_sha256.clone(),
            snapshot,
        ))
    }
}
