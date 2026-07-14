use super::super::*;

impl DarwinHostTransactionAdapter {
    pub(in super::super) fn create_journal(
        &self,
        record: TransactionJournal,
    ) -> Result<JournalSnapshot, DarwinHostError> {
        let bytes = journal_bytes(&record)?;
        let replacement = surface_tree(bytes);
        let journal_created = self
            .compare_exchange_tree(JOURNAL_PATH, None, Some(&replacement), None)
            .map_err(|_| DarwinHostError::new(DarwinHostErrorId::ConcurrentTransaction))?;
        if !journal_created {
            return Err(DarwinHostError::new(
                DarwinHostErrorId::ConcurrentTransaction,
            ));
        }
        let current = self
            .read_journal()?
            .ok_or_else(|| DarwinHostError::new(DarwinHostErrorId::ObservationChanged))?;
        if current.record != record {
            return Err(DarwinHostError::new(DarwinHostErrorId::ObservationChanged));
        }
        Ok(current)
    }

    pub(in super::super) fn read_journal(
        &self,
    ) -> Result<Option<JournalSnapshot>, DarwinHostError> {
        let Some(raw) = self.read_tree(JOURNAL_PATH, None, 1024 * 1024)? else {
            return Ok(None);
        };
        let record = decode_journal(&raw.bytes)?;
        let expected_before = if record.operation == DarwinHostOperation::RepairCache {
            1
        } else {
            DarwinHostSurface::ALL.len()
        };
        let surfaces_valid = if record.operation == DarwinHostOperation::RepairCache {
            record.before.first().map(|row| row.surface) == Some(DarwinHostSurface::Cache)
        } else {
            record
                .before
                .iter()
                .enumerate()
                .all(|(index, row)| row.surface as usize == index)
        };
        let phase_valid = match record.phase {
            JournalPhase::Pending => true,
            JournalPhase::Applying { step, .. } => {
                step == record.next_step && step < record.before.len()
            }
            JournalPhase::Cancelling { .. } => record.next_step == 0,
        };
        let digest_fields_valid = valid_digest(&record.root_id)
            && valid_digest(&record.plan_sha256)
            && record
                .lineage_before_sha256
                .as_deref()
                .is_none_or(valid_digest)
            && record
                .before
                .iter()
                .all(|row| row.tree_sha256.as_deref().is_none_or(valid_digest))
            && valid_target_binding(&record.target);
        if journal_bytes(&record).ok().as_deref() != Some(raw.bytes.as_slice())
            || record.schema != "harness-ultragoal.darwin-host-transaction-journal.v1"
            || record.root_id != self.root_id
            || record.marketplace != SUPPORTED_MARKETPLACE
            || record.generation == 0
            || (record.generation == 1) != record.lineage_before_sha256.is_none()
            || record.before.len() != expected_before
            || record.next_step > record.before.len()
            || !surfaces_valid
            || !phase_valid
            || !digest_fields_valid
        {
            return Err(DarwinHostError::new(DarwinHostErrorId::JournalCorrupt));
        }
        Ok(Some(JournalSnapshot {
            record,
            tree_sha256: raw.tree_sha256,
        }))
    }

    pub(in super::super) fn claim_step(
        &self,
        journal: JournalSnapshot,
        step: usize,
    ) -> Result<JournalSnapshot, DarwinHostError> {
        if journal.record.phase != JournalPhase::Pending
            || journal.record.next_step != step
            || step >= journal.record.before.len()
        {
            return Err(DarwinHostError::new(DarwinHostErrorId::JournalConflict));
        }
        let mut record = journal.record.clone();
        record.phase = JournalPhase::Applying {
            step,
            generation: 0,
        };
        self.replace_journal(journal, record, DarwinHostErrorId::JournalConflict)
    }

    pub(in super::super) fn reclaim_step(
        &self,
        journal: JournalSnapshot,
        step: usize,
    ) -> Result<JournalSnapshot, DarwinHostError> {
        let JournalPhase::Applying {
            step: claimed_step,
            generation,
        } = journal.record.phase
        else {
            return Err(DarwinHostError::new(DarwinHostErrorId::JournalConflict));
        };
        if claimed_step != step || journal.record.next_step != step {
            return Err(DarwinHostError::new(DarwinHostErrorId::JournalConflict));
        }
        let generation = generation
            .checked_add(1)
            .ok_or_else(|| DarwinHostError::new(DarwinHostErrorId::JournalConflict))?;
        let mut record = journal.record.clone();
        record.phase = JournalPhase::Applying { step, generation };
        self.replace_journal(journal, record, DarwinHostErrorId::JournalConflict)
    }

    pub(in super::super) fn finish_step(
        &self,
        journal: JournalSnapshot,
        step: usize,
    ) -> Result<JournalSnapshot, DarwinHostError> {
        if !matches!(
            journal.record.phase,
            JournalPhase::Applying {
                step: claimed_step,
                ..
            } if claimed_step == step
        ) || journal.record.next_step != step
            || step >= journal.record.before.len()
        {
            return Err(DarwinHostError::new(DarwinHostErrorId::JournalConflict));
        }
        let mut record = journal.record.clone();
        record.next_step = step + 1;
        record.phase = JournalPhase::Pending;
        self.replace_journal(journal, record, DarwinHostErrorId::JournalConflict)
    }
}
