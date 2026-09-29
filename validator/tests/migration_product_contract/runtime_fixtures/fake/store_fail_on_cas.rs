impl FakeStore {
    pub(crate) fn fail_on_cas(&self, number: usize) {
        self.state.lock().unwrap().fail_on_cas = Some(number);
    }

    pub(crate) fn operation_count(&self) -> usize {
        self.state.lock().unwrap().operations.len()
    }

    pub(crate) fn only_operation(&self) -> MigrationOperation {
        let state = self.state.lock().unwrap();
        assert_eq!(state.operations.len(), 1);
        state.operations.values().next().unwrap().clone()
    }

    pub(crate) fn substitute_only_operation_from_json(&self, value: Value) {
        let operation: MigrationOperation = serde_json::from_value(value).unwrap();
        let mut state = self.state.lock().unwrap();
        assert_eq!(state.operations.len(), 1);
        let operation_id = state.operations.keys().next().unwrap().clone();
        state.operations.insert(operation_id, operation);
    }
}

impl DurableMigrationStore for FakeStore {
    fn register_authorization(
        &self,
        record: &crate::migration::product::AuthorizationRecord,
    ) -> Result<(), StoreFault> {
        if !record.validate_shape() {
            return Err(StoreFault::new("test-authorization-invalid"));
        }
        let mut state = self.state.lock().unwrap();
        match state.authorizations.get(record.authorization_id()) {
            Some(existing) if existing == record => Ok(()),
            Some(_) => Err(StoreFault::new("test-authorization-conflict")),
            None => {
                state
                    .authorizations
                    .insert(record.authorization_id().to_owned(), record.clone());
                Ok(())
            }
        }
    }

    fn reserve_once(
        &self,
        request: &ReservationRequest,
        initial: &MigrationOperation,
    ) -> Result<ReservationResult, StoreFault> {
        if !request.validate_shape() || !initial.validate_shape() {
            return Err(StoreFault::new("test-reservation-invalid"));
        }
        let mut state = self.state.lock().unwrap();
        let registered = state
            .authorizations
            .get(request.authorization().authorization_id())
            .ok_or_else(|| StoreFault::new("test-authorization-unregistered"))?;
        if registered != request.authorization() {
            return Err(StoreFault::new("test-authorization-substituted"));
        }
        if state
            .consumed
            .contains(request.authorization().authorization_id())
        {
            return state
                .operations
                .get(request.operation_id())
                .cloned()
                .map(ReservationResult::Existing)
                .ok_or_else(|| StoreFault::new("test-authorization-replayed"));
        }
        for key in request.semantic_keys() {
            if state
                .reservations
                .get(key)
                .is_some_and(|owner| owner != request.operation_id())
            {
                return Err(StoreFault::new("test-semantic-reservation-conflict"));
            }
        }
        state
            .consumed
            .insert(request.authorization().authorization_id().to_owned());
        for key in request.semantic_keys() {
            state
                .reservations
                .insert(key.clone(), request.operation_id().to_owned());
        }
        state
            .operations
            .insert(request.operation_id().to_owned(), initial.clone());
        Ok(ReservationResult::Created(initial.clone()))
    }

    fn load_operation(&self, operation_id: &str) -> Result<Option<MigrationOperation>, StoreFault> {
        Ok(self
            .state
            .lock()
            .unwrap()
            .operations
            .get(operation_id)
            .cloned())
    }

    fn compare_and_swap(
        &self,
        operation_id: &str,
        expected_revision: u64,
        expected_journal_sha256: &str,
        next: &MigrationOperation,
    ) -> Result<MigrationOperation, StoreFault> {
        let mut state = self.state.lock().unwrap();
        state.cas_count += 1;
        if state.fail_on_cas == Some(state.cas_count) {
            state.fail_on_cas = None;
            return Err(StoreFault::new("test-injected-crash"));
        }
        let current = state
            .operations
            .get(operation_id)
            .ok_or_else(|| StoreFault::new("test-operation-missing"))?;
        if current.revision() != expected_revision
            || current.journal_sha256() != expected_journal_sha256
            || next.operation_id() != operation_id
            || next.revision() != expected_revision + 1
            || !next.validate_shape()
        {
            return Err(StoreFault::new("test-cas-conflict"));
        }
        state
            .operations
            .insert(operation_id.to_owned(), next.clone());
        Ok(next.clone())
    }
}

#[derive(Default)]
struct EffectState {
    authorities: BTreeMap<String, AuthoritySnapshot>,
    effect_permit_sha256: BTreeMap<String, Option<String>>,
    apply_count: usize,
    rollback_count: usize,
    reject_effect_id: Option<String>,
    reject_after_effect_id: Option<String>,
    ambiguous_effect_id: Option<String>,
    compatibility_prerequisite_inputs: Vec<String>,
}

#[derive(Clone, Default)]
pub(crate) struct FakeEffects {
    state: Arc<Mutex<EffectState>>,
}
