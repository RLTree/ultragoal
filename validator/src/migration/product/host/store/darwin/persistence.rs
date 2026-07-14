pub(crate) struct DarwinMigrationStore {
    context: Arc<HostContext>,
}

impl DarwinMigrationStore {
    pub(super) fn new(context: Arc<HostContext>) -> Result<Self, HostError> {
        let value = Self { context };
        let _guard = value
            .context
            .io()
            .lock()
            .map_err(|_| HostError::new("migration-host-lock-poisoned"))?;
        read_ledger(&value.context)?;
        drop(_guard);
        Ok(value)
    }

    #[cfg(test)]
    pub(crate) fn fail_on_cas_for_test(&self, number: usize) {
        *self
            .context
            .fail_on_cas
            .lock()
            .unwrap_or_else(|poison| poison.into_inner()) = Some(number);
        self.context
            .cas_count
            .store(0, std::sync::atomic::Ordering::SeqCst);
    }

    #[cfg(test)]
    pub(crate) fn effect_counts_for_test(&self) -> (u64, u64) {
        let _guard = self.context.io().lock().unwrap();
        read_ledger(&self.context).unwrap().counts()
    }

    #[cfg(test)]
    pub(crate) fn only_operation_for_test(&self) -> MigrationOperation {
        let _guard = self.context.io().lock().unwrap();
        let ledger = read_ledger(&self.context).unwrap();
        assert_eq!(ledger.operations.len(), 1);
        ledger.operations.values().next().unwrap().clone()
    }
}
