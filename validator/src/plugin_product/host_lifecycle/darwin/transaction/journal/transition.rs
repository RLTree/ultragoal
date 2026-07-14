use super::super::*;

impl DarwinHostTransactionAdapter {
    pub(in super::super) fn claim_cancellation(
        &self,
        journal: JournalSnapshot,
    ) -> Result<JournalSnapshot, DarwinHostError> {
        if journal.record.next_step != 0 {
            return Err(DarwinHostError::new(DarwinHostErrorId::CancellationUnsafe));
        }
        let generation = match journal.record.phase {
            JournalPhase::Pending => 0,
            JournalPhase::Cancelling { generation } => generation
                .checked_add(1)
                .ok_or_else(|| DarwinHostError::new(DarwinHostErrorId::CancellationUnsafe))?,
            JournalPhase::Applying { step, .. } => {
                let surface = journal
                    .record
                    .before
                    .get(step)
                    .map(|row| row.surface)
                    .ok_or_else(|| DarwinHostError::new(DarwinHostErrorId::JournalCorrupt))?;
                return Err(DarwinHostError::at(
                    DarwinHostErrorId::CancellationUnsafe,
                    surface,
                ));
            }
        };
        let mut record = journal.record.clone();
        record.phase = JournalPhase::Cancelling { generation };
        self.replace_journal(journal, record, DarwinHostErrorId::CancellationUnsafe)
    }

    pub(in super::super) fn replace_journal(
        &self,
        journal: JournalSnapshot,
        record: TransactionJournal,
        conflict: DarwinHostErrorId,
    ) -> Result<JournalSnapshot, DarwinHostError> {
        let replacement = surface_tree(journal_bytes(&record)?);
        let journal_replaced = self
            .compare_exchange_tree(
                JOURNAL_PATH,
                Some(&journal.tree_sha256),
                Some(&replacement),
                None,
            )
            .map_err(|_| DarwinHostError::new(conflict))?;
        if !journal_replaced {
            return Err(DarwinHostError::new(conflict));
        }
        let current = self
            .read_journal()?
            .ok_or_else(|| DarwinHostError::new(conflict))?;
        if current.record != record {
            return Err(DarwinHostError::new(conflict));
        }
        Ok(current)
    }

    pub(in super::super) fn remove_journal(
        &self,
        journal: &JournalSnapshot,
    ) -> Result<(), DarwinHostError> {
        let journal_removed = self
            .compare_exchange_tree(JOURNAL_PATH, Some(&journal.tree_sha256), None, None)
            .map_err(|_| DarwinHostError::new(DarwinHostErrorId::JournalConflict))?;
        if !journal_removed {
            return Err(DarwinHostError::new(DarwinHostErrorId::JournalConflict));
        }
        if self.read_journal()?.is_some() {
            return Err(DarwinHostError::new(DarwinHostErrorId::ObservationChanged));
        }
        Ok(())
    }

    pub(in super::super) fn compare_exchange_tree(
        &self,
        relative: &str,
        expected: Option<&str>,
        replacement: Option<&[TreeObject]>,
        surface: Option<DarwinHostSurface>,
    ) -> Result<bool, DarwinHostError> {
        let mut tree = ScopedTree::new(self.root.clone(), relative)
            .map_err(|error| map_surface_distribution(error, surface))?;
        MaterializeEffects::compare_exchange_tree(&mut tree, expected, replacement)
            .map_err(|_| surface_error(DarwinHostErrorId::EffectFailed, surface))
    }

    pub(in super::super) fn require_plan(
        &self,
        plan: &DarwinHostTransactionPlan,
    ) -> Result<(), DarwinHostError> {
        if plan.root_id != self.root_id || plan.marketplace != SUPPORTED_MARKETPLACE {
            return Err(DarwinHostError::new(DarwinHostErrorId::ConfinementRejected));
        }
        Ok(())
    }

    pub(in super::super) fn require_journal(
        &self,
        plan: &DarwinHostTransactionPlan,
        journal: &TransactionJournal,
    ) -> Result<(), DarwinHostError> {
        if journal.root_id != self.root_id
            || journal.plan_sha256 != plan.plan_sha256
            || journal.operation != plan.operation
            || journal.target != plan.target
            || journal.marketplace != plan.marketplace
            || journal.before.len() != plan.steps.len()
            || journal
                .before
                .iter()
                .zip(&plan.steps)
                .any(|(before, step)| before.surface != step.surface)
        {
            return Err(DarwinHostError::new(DarwinHostErrorId::JournalConflict));
        }
        Ok(())
    }
}
