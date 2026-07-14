impl DurableMigrationStore for DarwinMigrationStore {
    fn register_authorization(&self, record: &AuthorizationRecord) -> Result<(), StoreFault> {
        if !record.validate_shape() {
            return Err(StoreFault::new("migration-host-authorization-invalid"));
        }
        let _guard = self
            .context
            .io()
            .lock()
            .map_err(|_| StoreFault::new("migration-host-lock-poisoned"))?;
        let mut ledger = read_ledger(&self.context).map_err(store_fault)?;
        match ledger.authorizations.get(record.authorization_id()) {
            Some(existing) if existing == record => return Ok(()),
            Some(_) => return Err(StoreFault::new("migration-host-authorization-conflict")),
            None => {}
        }
        if ledger.authorizations.len() >= MAX_LEDGER_ROWS {
            return Err(StoreFault::new("migration-host-ledger-capacity-refused"));
        }
        ledger
            .authorizations
            .insert(record.authorization_id().to_owned(), record.clone());
        ledger.advance().map_err(store_fault)?;
        write_ledger(&self.context, &ledger).map_err(store_fault)
    }

    fn reserve_once(
        &self,
        request: &ReservationRequest,
        initial: &MigrationOperation,
    ) -> Result<ReservationResult, StoreFault> {
        if !request.validate_shape()
            || !initial.validate_shape()
            || request.operation_id() != initial.operation_id()
        {
            return Err(StoreFault::new("migration-host-reservation-invalid"));
        }
        let _guard = self
            .context
            .io()
            .lock()
            .map_err(|_| StoreFault::new("migration-host-lock-poisoned"))?;
        let mut ledger = read_ledger(&self.context).map_err(store_fault)?;
        let registered = ledger
            .authorizations
            .get(request.authorization().authorization_id())
            .ok_or_else(|| StoreFault::new("migration-host-authorization-unregistered"))?;
        if registered != request.authorization() {
            return Err(StoreFault::new("migration-host-authorization-substituted"));
        }
        if let Some(operation_id) = ledger
            .consumed_authorizations
            .get(request.authorization().authorization_id())
        {
            return ledger
                .operations
                .get(operation_id)
                .cloned()
                .map(ReservationResult::Existing)
                .ok_or_else(|| StoreFault::new("migration-host-consumed-operation-missing"));
        }
        if request.semantic_keys().iter().any(|key| {
            ledger
                .semantic_reservations
                .get(key)
                .is_some_and(|owner| owner != request.operation_id())
        }) {
            return Err(StoreFault::new(
                "migration-host-semantic-reservation-conflict",
            ));
        }
        if ledger.operations.len() >= MAX_LEDGER_ROWS {
            return Err(StoreFault::new("migration-host-ledger-capacity-refused"));
        }
        ledger.consumed_authorizations.insert(
            request.authorization().authorization_id().to_owned(),
            request.operation_id().to_owned(),
        );
        for key in request.semantic_keys() {
            ledger
                .semantic_reservations
                .insert(key.clone(), request.operation_id().to_owned());
        }
        ledger
            .operations
            .insert(request.operation_id().to_owned(), initial.clone());
        ledger.advance().map_err(store_fault)?;
        write_ledger(&self.context, &ledger).map_err(store_fault)?;
        Ok(ReservationResult::Created(initial.clone()))
    }

    fn load_operation(&self, operation_id: &str) -> Result<Option<MigrationOperation>, StoreFault> {
        if !super::super::super::valid_sha256(operation_id) {
            return Err(StoreFault::new("migration-host-operation-id-invalid"));
        }
        let _guard = self
            .context
            .io()
            .lock()
            .map_err(|_| StoreFault::new("migration-host-lock-poisoned"))?;
        let ledger = read_ledger(&self.context).map_err(store_fault)?;
        Ok(ledger.operations.get(operation_id).cloned())
    }

    fn compare_and_swap(
        &self,
        operation_id: &str,
        expected_revision: u64,
        expected_journal_sha256: &str,
        next: &MigrationOperation,
    ) -> Result<MigrationOperation, StoreFault> {
        #[cfg(test)]
        {
            let count = self
                .context
                .cas_count
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
                + 1;
            let mut fail = self
                .context
                .fail_on_cas
                .lock()
                .unwrap_or_else(|poison| poison.into_inner());
            if *fail == Some(count) {
                *fail = None;
                return Err(StoreFault::new("migration-host-injected-interruption"));
            }
        }
        let _guard = self
            .context
            .io()
            .lock()
            .map_err(|_| StoreFault::new("migration-host-lock-poisoned"))?;
        let mut ledger = read_ledger(&self.context).map_err(store_fault)?;
        let current = ledger
            .operations
            .get(operation_id)
            .ok_or_else(|| StoreFault::new("migration-host-operation-missing"))?;
        let next_revision = expected_revision
            .checked_add(1)
            .ok_or_else(|| StoreFault::new("migration-host-journal-revision-exhausted"))?;
        if current.revision() != expected_revision
            || current.journal_sha256() != expected_journal_sha256
            || next.operation_id() != operation_id
            || next.revision() != next_revision
            || !next.validate_shape()
        {
            return Err(StoreFault::new("migration-host-journal-cas-conflict"));
        }
        ledger
            .operations
            .insert(operation_id.to_owned(), next.clone());
        ledger.advance().map_err(store_fault)?;
        write_ledger(&self.context, &ledger).map_err(store_fault)?;
        Ok(next.clone())
    }
}

pub(super) fn read_ledger(context: &HostContext) -> Result<HostLedger, HostError> {
    context.verify_static()?;
    let observed = context
        .state()
        .read_regular(LEDGER_NAME, true, MAX_HOST_FILE_BYTES)?;
    let ledger: HostLedger = serde_json::from_slice(&observed.bytes)
        .map_err(|_| HostError::new("migration-host-ledger-invalid"))?;
    ledger.validate(context.scope_id())?;
    if ledger.canonical_bytes()? != observed.bytes {
        return Err(HostError::new("migration-host-ledger-noncanonical"));
    }
    context.verify_static()?;
    Ok(ledger)
}

pub(super) fn write_ledger(context: &HostContext, ledger: &HostLedger) -> Result<(), HostError> {
    ledger.validate(context.scope_id())?;
    let bytes = ledger.canonical_bytes()?;
    if bytes.len() as u64 > MAX_HOST_FILE_BYTES {
        return Err(HostError::new("migration-host-ledger-size-refused"));
    }
    context.state().write_atomic(LEDGER_NAME, &bytes)?;
    let reread = read_ledger(context)?;
    if &reread != ledger {
        return Err(HostError::new("migration-host-ledger-publish-substituted"));
    }
    Ok(())
}
