trait TestSurfaceSource {
    fn provenance_sha256(&self) -> &str;
    fn generation(&self) -> u64 {
        0
    }
    fn transaction_supported(&self) -> bool {
        true
    }
    fn read_marketplace(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()>;
    fn read_cache(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()>;
    fn read_registry(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()>;
    fn read_plugins_ui(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()>;
    fn read_runtime(&mut self) -> Result<Option<crate::distribution::RuntimeObservation>, ()>;
}

struct TestSurfaceTransaction<'a, S> {
    source: &'a mut S,
    host_scope_sha256: String,
    session_issuance_sha256: String,
    start_generation: u64,
}

impl<S: TestSurfaceSource> HostSurfaceTransaction for TestSurfaceTransaction<'_, S> {
    fn provenance_sha256(&self) -> &str {
        self.source.provenance_sha256()
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
        Ok(self.source.generation())
    }

    fn read_marketplace(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        self.source.read_marketplace(maximum)
    }

    fn read_cache(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        self.source.read_cache(maximum)
    }

    fn read_registry(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        self.source.read_registry(maximum)
    }

    fn read_plugins_ui(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        self.source.read_plugins_ui(maximum)
    }

    fn read_runtime(&mut self) -> Result<Option<crate::distribution::RuntimeObservation>, ()> {
        self.source.read_runtime()
    }
}

fn with_test_transaction<S: TestSurfaceSource, T>(
    source: &mut S,
    request: &HostObservationTransactionRequest,
    operation: impl FnOnce(Result<&mut dyn HostSurfaceTransaction, HostSurfaceTransactionError>) -> T,
) -> T {
    if !source.transaction_supported() {
        return operation(Err(HostSurfaceTransactionError::Unsupported));
    }
    let start_generation = source.generation();
    let mut transaction = TestSurfaceTransaction {
        source,
        host_scope_sha256: request.host_scope_sha256().to_owned(),
        session_issuance_sha256: request.session_issuance_sha256().to_owned(),
        start_generation,
    };
    operation(Ok(&mut transaction))
}

#[derive(Debug, Default)]
struct AdapterInvocationCounts {
    transactions: std::cell::Cell<usize>,
    provenance: std::cell::Cell<usize>,
    scope: std::cell::Cell<usize>,
    issuance: std::cell::Cell<usize>,
    start_generation: std::cell::Cell<usize>,
    current_generation: std::cell::Cell<usize>,
    marketplace_reads: std::cell::Cell<usize>,
    cache_reads: std::cell::Cell<usize>,
    registry_reads: std::cell::Cell<usize>,
    ui_reads: std::cell::Cell<usize>,
    runtime_reads: std::cell::Cell<usize>,
}

impl AdapterInvocationCounts {
    fn increment(cell: &std::cell::Cell<usize>) {
        cell.set(cell.get() + 1);
    }

    fn is_zero(&self) -> bool {
        [
            &self.transactions,
            &self.provenance,
            &self.scope,
            &self.issuance,
            &self.start_generation,
            &self.current_generation,
            &self.marketplace_reads,
            &self.cache_reads,
            &self.registry_reads,
            &self.ui_reads,
            &self.runtime_reads,
        ]
        .into_iter()
        .all(|value| value.get() == 0)
    }

    fn read_count(&self) -> usize {
        self.marketplace_reads.get()
            + self.cache_reads.get()
            + self.registry_reads.get()
            + self.ui_reads.get()
            + self.runtime_reads.get()
    }
}

#[derive(Default)]
struct CountingUnsupportedReader {
    counts: AdapterInvocationCounts,
}

impl HostSurfaceReader for CountingUnsupportedReader {
    fn with_transaction<T>(
        &mut self,
        _request: &HostObservationTransactionRequest,
        operation: impl FnOnce(
            Result<&mut dyn HostSurfaceTransaction, HostSurfaceTransactionError>,
        ) -> T,
    ) -> T {
        AdapterInvocationCounts::increment(&self.counts.transactions);
        operation(Err(HostSurfaceTransactionError::Unsupported))
    }
}

struct CountingMutationReader {
    inner: Reader,
    counts: AdapterInvocationCounts,
    before_transaction: Option<Box<dyn FnOnce()>>,
}

impl HostSurfaceReader for CountingMutationReader {
    fn with_transaction<T>(
        &mut self,
        request: &HostObservationTransactionRequest,
        operation: impl FnOnce(
            Result<&mut dyn HostSurfaceTransaction, HostSurfaceTransactionError>,
        ) -> T,
    ) -> T {
        AdapterInvocationCounts::increment(&self.counts.transactions);
        if let Some(mutation) = self.before_transaction.take() {
            mutation();
        }
        let start_generation = self.inner.generation;
        let mut transaction = CountingSurfaceTransaction {
            reader: &mut self.inner,
            counts: &self.counts,
            host_scope_sha256: request.host_scope_sha256().to_owned(),
            session_issuance_sha256: request.session_issuance_sha256().to_owned(),
            start_generation,
        };
        operation(Ok(&mut transaction))
    }
}

struct CountingSurfaceTransaction<'a> {
    reader: &'a mut Reader,
    counts: &'a AdapterInvocationCounts,
    host_scope_sha256: String,
    session_issuance_sha256: String,
    start_generation: u64,
}
