impl Ledger {
    pub(super) fn require_issued(&self, permit_id: &str) -> Result<(), ProductError> {
        let _lock = self.store.lock()?;
        self.store.verify()?;
        let records = self.read_records()?;
        self.require_current_observation(&records)?;
        require_state(&records, permit_id, Some(LedgerState::Issued))
    }

    pub(super) fn open(store: Store) -> Result<Self, ProductError> {
        let ledger = Self {
            store,
            observation: Mutex::new(LedgerObservation {
                sequence: 0,
                record_id: None,
                byte_length: 0,
                changed_seconds: 0,
                changed_nanoseconds: 0,
            }),
        };
        let store_lock = ledger.store.lock()?;
        ledger.store.verify()?;
        let records = ledger.read_records()?;
        let observed = observe_ledger(&records, ledger.store.ledger_stamp()?);
        if !records.is_empty() {
            return Err(ProductError::AuthorityCheckpointRequired);
        }
        *ledger.observation_lock()? = observed;
        drop(store_lock);
        Ok(ledger)
    }

    fn observation_lock(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, LedgerObservation>, ProductError> {
        self.observation
            .lock()
            .map_err(|_| ProductError::AuthorityStoreInvalid)
    }

    fn require_current_observation(&self, records: &[LedgerRecord]) -> Result<(), ProductError> {
        if *self.observation_lock()? == observe_ledger(records, self.store.ledger_stamp()?) {
            Ok(())
        } else {
            Err(ProductError::AuthorityCheckpointRequired)
        }
    }
}

fn observe_ledger(
    records: &[LedgerRecord],
    (byte_length, changed_seconds, changed_nanoseconds): (u64, i64, i64),
) -> LedgerObservation {
    LedgerObservation {
        sequence: records.len() as u64,
        record_id: records.last().map(|record| record.record_id.clone()),
        byte_length,
        changed_seconds,
        changed_nanoseconds,
    }
}
