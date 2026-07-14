const OBSERVATION_LIMIT: usize = 4 * 1024 * 1024;

pub trait HostSurfaceReader {
    fn with_transaction<T>(
        &mut self,
        request: &HostObservationTransactionRequest,
        operation: impl FnOnce(
            Result<&mut dyn HostSurfaceTransaction, HostSurfaceTransactionError>,
        ) -> T,
    ) -> T;
}

pub trait HostSurfaceTransaction {
    fn provenance_sha256(&self) -> &str;
    fn host_scope_sha256(&self) -> &str;
    fn session_issuance_sha256(&self) -> &str;
    fn start_generation(&self) -> u64;
    fn current_generation(&self) -> Result<u64, ()>;
    fn read_marketplace(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()>;
    fn read_cache(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()>;
    fn read_registry(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()>;
    fn read_plugins_ui(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()>;
    fn read_runtime(&mut self) -> Result<Option<RuntimeObservation>, ()>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HostSurfaceTransactionError {
    Unsupported,
    Failed,
}

pub struct HostObservationTransactionRequest {
    provenance_sha256: String,
    host_scope_sha256: String,
    session_issuance_sha256: String,
}

impl HostObservationTransactionRequest {
    pub fn provenance_sha256(&self) -> &str {
        &self.provenance_sha256
    }

    pub fn host_scope_sha256(&self) -> &str {
        &self.host_scope_sha256
    }

    pub fn session_issuance_sha256(&self) -> &str {
        &self.session_issuance_sha256
    }

    pub(super) fn issue(
        provenance_sha256: &str,
        host_scope: &Arc<BoundHostScope>,
        issuance: &Arc<SessionIssuance>,
    ) -> Self {
        Self {
            provenance_sha256: provenance_sha256.to_owned(),
            host_scope_sha256: host_scope.scope_sha256().to_owned(),
            session_issuance_sha256: issuance.issuance_sha256().to_owned(),
        }
    }
}

pub(super) struct BoundHostObservationTransaction<'a> {
    transaction: &'a mut dyn HostSurfaceTransaction,
    host_scope: Arc<BoundHostScope>,
    issuance: Arc<SessionIssuance>,
    provenance_sha256: String,
    start_generation: u64,
}

impl<'a> BoundHostObservationTransaction<'a> {
    pub(super) fn bind(
        transaction: &'a mut dyn HostSurfaceTransaction,
        request: &HostObservationTransactionRequest,
        host_scope: &Arc<BoundHostScope>,
        issuance: &Arc<SessionIssuance>,
    ) -> Result<Self, HostLifecycleError> {
        host_scope.revalidate()?;
        issuance.require_observation_ready()?;
        let start_generation = transaction.start_generation();
        let bound = Self {
            transaction,
            host_scope: Arc::clone(host_scope),
            issuance: Arc::clone(issuance),
            provenance_sha256: request.provenance_sha256().to_owned(),
            start_generation,
        };
        bound.require_current(request)?;
        Ok(bound)
    }

    pub(super) fn require_current(
        &self,
        request: &HostObservationTransactionRequest,
    ) -> Result<(), HostLifecycleError> {
        self.host_scope.revalidate()?;
        self.issuance.require_observation_ready()?;
        let current_generation = self
            .transaction
            .current_generation()
            .map_err(|_| unavailable())?;
        if self.transaction.provenance_sha256() != self.provenance_sha256
            || self.transaction.provenance_sha256() != request.provenance_sha256()
            || self.transaction.host_scope_sha256() != request.host_scope_sha256()
            || self.transaction.session_issuance_sha256() != request.session_issuance_sha256()
            || request.host_scope_sha256() != self.host_scope.scope_sha256()
            || request.session_issuance_sha256() != self.issuance.issuance_sha256()
            || self.transaction.start_generation() != self.start_generation
            || current_generation != self.start_generation
        {
            return Err(HostLifecycleError::new(
                HostLifecycleErrorId::ObservationChanged,
            ));
        }
        Ok(())
    }

    pub(super) fn transaction_provenance_sha256(&self) -> &str {
        self.transaction.provenance_sha256()
    }

    pub(super) fn capture(
        &mut self,
        marketplace_plan: &MarketplacePlan,
        binding: &JourneyBinding,
        teardown: bool,
        request: &HostObservationTransactionRequest,
    ) -> Result<HostObservationFrame, HostLifecycleError> {
        self.require_current(request)?;
        let result = capture_host_observations(
            self.transaction,
            marketplace_plan,
            binding,
            &self.host_scope,
            &self.issuance,
            teardown,
        );
        self.require_current(request)?;
        result
    }

    pub(super) fn commit<T>(
        self,
        request: &HostObservationTransactionRequest,
        report: T,
    ) -> Result<T, HostLifecycleError> {
        self.require_current(request)?;
        self.issuance.close()?;
        Ok(report)
    }
}

struct HostObservationExpectations<'a> {
    marketplace_plan: &'a MarketplacePlan,
    binding: &'a JourneyBinding,
    host_scope: &'a Arc<BoundHostScope>,
    issuance: &'a Arc<SessionIssuance>,
    teardown: bool,
}

pub struct HostObservationFrame {
    binding_sha256: String,
    host_scope: Arc<BoundHostScope>,
    host_scope_sha256: String,
    session_issuance: Arc<SessionIssuance>,
    session_issuance_sha256: String,
    reader_provenance_sha256: String,
    raw_observation_sha256: String,
    layers: Vec<HostLayerReport>,
    pub(super) marketplace: Option<MarketplaceSnapshot>,
    pub(super) cache: Option<CacheSnapshot>,
    pub(super) app_registry: Option<AppRegistryObservation>,
    pub(super) plugins_ui: PluginsUiObservation,
    pub(super) discovery: Option<DiscoveryObservation>,
    pub(super) runtime: Option<RuntimeObservation>,
}
