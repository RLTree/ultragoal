impl HostSurfaceReader for TransactionFailureReader {
    fn with_transaction<T>(
        &mut self,
        _request: &HostObservationTransactionRequest,
        operation: impl FnOnce(
            Result<&mut dyn HostSurfaceTransaction, HostSurfaceTransactionError>,
        ) -> T,
    ) -> T {
        operation(Err(self.error))
    }
}

struct PanickingTransactionReader {
    inner: Reader,
}

impl HostSurfaceReader for PanickingTransactionReader {
    fn with_transaction<T>(
        &mut self,
        request: &HostObservationTransactionRequest,
        operation: impl FnOnce(
            Result<&mut dyn HostSurfaceTransaction, HostSurfaceTransactionError>,
        ) -> T,
    ) -> T {
        with_test_transaction(self, request, operation)
    }
}

impl TestSurfaceSource for PanickingTransactionReader {
    fn provenance_sha256(&self) -> &str {
        &self.inner.provenance_sha256
    }

    fn read_marketplace(&mut self, _maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        panic!("transaction-reader-panic-canary")
    }

    fn read_cache(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        let _ = maximum;
        self.inner.read_cache_raw()
    }

    fn read_registry(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        let _ = maximum;
        self.inner.read_registry_raw()
    }

    fn read_plugins_ui(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        let _ = maximum;
        self.inner.read_plugins_ui_raw()
    }

    fn read_runtime(&mut self) -> Result<Option<crate::distribution::RuntimeObservation>, ()> {
        self.inner.read_runtime_raw()
    }
}

#[test]
fn unsupported_failed_and_panicking_transactions_are_bounded_and_do_not_close() {
    let fixture = Fixture::new("transaction-failure-cleanup");
    let bundle = fixture.bundle("0.0.12");
    fixture.seed(Some(&bundle), Some(&bundle));
    let state = installed(&bundle.authority, 16);
    let repeat = lifecycle(
        &state,
        request(
            LifecycleIntent::RepeatUse,
            Some(bundle.authority.clone()),
            None,
            state.installed.as_ref(),
            false,
            false,
        ),
    );
    let (mut session, host) = fixture.session(&bundle, repeat);
    session.apply_confined(&state).unwrap();
    let before = fixture.tree();

    let mut unsupported = TransactionFailureReader {
        error: HostSurfaceTransactionError::Unsupported,
    };
    let error = session.capture_and_verify(&mut unsupported).unwrap_err();
    assert_eq!(
        error.id(),
        HostLifecycleErrorId::ObservationTransactionUnsupported
    );
    assert!(!error.to_string().contains("canary"));

    let mut failed = TransactionFailureReader {
        error: HostSurfaceTransactionError::Failed,
    };
    let error = session.capture_and_verify(&mut failed).unwrap_err();
    assert_eq!(error.id(), HostLifecycleErrorId::ObservationUnavailable);

    let mut panicking = PanickingTransactionReader {
        inner: Reader::complete(&bundle, &host, session.binding()),
    };
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = session.capture_and_verify(&mut panicking);
    }));
    assert!(panic.is_err());
    assert_eq!(fixture.tree(), before);

    let mut current = Reader::complete(&bundle, &host, session.binding());
    assert!(session.capture_and_verify(&mut current).is_ok());
    assert_eq!(
        session.capture_and_verify(&mut current).unwrap_err().id(),
        HostLifecycleErrorId::SessionStateRejected
    );
    assert_eq!(fixture.tree(), before);
}

struct LockedSurfaceState {
    reader: Reader,
    generation: u64,
}

struct LockedSurfaceReader {
    state: std::sync::Arc<std::sync::Mutex<LockedSurfaceState>>,
    writer_ready: std::sync::Arc<std::sync::Barrier>,
}

struct LockedSurfaceTransaction<'a> {
    state: std::sync::MutexGuard<'a, LockedSurfaceState>,
    host_scope_sha256: String,
    session_issuance_sha256: String,
    start_generation: u64,
}

impl HostSurfaceReader for LockedSurfaceReader {
    fn with_transaction<T>(
        &mut self,
        request: &HostObservationTransactionRequest,
        operation: impl FnOnce(
            Result<&mut dyn HostSurfaceTransaction, HostSurfaceTransactionError>,
        ) -> T,
    ) -> T {
        let state = self.state.lock().unwrap();
        self.writer_ready.wait();
        let start_generation = state.generation;
        let mut transaction = LockedSurfaceTransaction {
            state,
            host_scope_sha256: request.host_scope_sha256().to_owned(),
            session_issuance_sha256: request.session_issuance_sha256().to_owned(),
            start_generation,
        };
        operation(Ok(&mut transaction))
    }
}

impl HostSurfaceTransaction for LockedSurfaceTransaction<'_> {
    fn provenance_sha256(&self) -> &str {
        &self.state.reader.provenance_sha256
    }

    fn host_scope_sha256(&self) -> &str {
        &self.host_scope_sha256
    }

    fn session_issuance_sha256(&self) -> &str {
        &self.session_issuance_sha256
    }

    fn start_generation(&self) -> u64 {
        self.start_generation
    }

    fn current_generation(&self) -> Result<u64, ()> {
        Ok(self.state.generation)
    }

    fn read_marketplace(&mut self, _maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        self.state.reader.read_marketplace_raw()
    }

    fn read_cache(&mut self, _maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        self.state.reader.read_cache_raw()
    }

    fn read_registry(&mut self, _maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        self.state.reader.read_registry_raw()
    }

    fn read_plugins_ui(&mut self, _maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        self.state.reader.read_plugins_ui_raw()
    }

    fn read_runtime(&mut self) -> Result<Option<crate::distribution::RuntimeObservation>, ()> {
        self.state.reader.read_runtime_raw()
    }
}
