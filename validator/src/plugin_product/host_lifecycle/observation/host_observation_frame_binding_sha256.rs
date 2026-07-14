impl HostObservationFrame {
    pub(super) fn binding_sha256(&self) -> &str {
        &self.binding_sha256
    }

    pub(super) fn host_scope_sha256(&self) -> &str {
        &self.host_scope_sha256
    }

    pub(super) fn session_issuance_sha256(&self) -> &str {
        &self.session_issuance_sha256
    }

    pub(super) fn reader_provenance_sha256(&self) -> &str {
        &self.reader_provenance_sha256
    }

    pub(super) fn layers(&self) -> &[HostLayerReport] {
        &self.layers
    }

    pub(super) fn layer(&self, layer: HostLayer) -> &HostLayerReport {
        self.layers
            .iter()
            .find(|row| row.layer() == layer)
            .expect("complete observed host layer set")
    }

    pub(super) fn plugins_ui(&self) -> &PluginsUiObservation {
        &self.plugins_ui
    }

    pub(super) fn require_exact_session(
        &self,
        binding: &JourneyBinding,
        host_scope: &Arc<BoundHostScope>,
        issuance: &Arc<SessionIssuance>,
    ) -> Result<(), HostLifecycleError> {
        if self.binding_sha256 != binding.binding_sha256()
            || self.host_scope_sha256 != host_scope.scope_sha256()
            || self.session_issuance_sha256 != issuance.issuance_sha256()
            || !same_scope(&self.host_scope, host_scope)
            || !same_issuance(&self.session_issuance, issuance)
        {
            return Err(HostLifecycleError::new(
                HostLifecycleErrorId::IdentityMismatch,
            ));
        }
        self.host_scope.revalidate()?;
        host_scope.revalidate()?;
        issuance.require_observation_ready()
    }

    pub(super) fn same_observation(&self, fresh: &Self) -> bool {
        self.reader_provenance_sha256 == fresh.reader_provenance_sha256
            && self.raw_observation_sha256 == fresh.raw_observation_sha256
            && self.layers == fresh.layers
            && self.binding_sha256 == fresh.binding_sha256
            && self.host_scope_sha256 == fresh.host_scope_sha256
            && self.session_issuance_sha256 == fresh.session_issuance_sha256
            && same_scope(&self.host_scope, &fresh.host_scope)
            && same_issuance(&self.session_issuance, &fresh.session_issuance)
    }
}

impl fmt::Debug for HostObservationFrame {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HostObservationFrame")
            .field("binding_sha256", &self.binding_sha256)
            .field("host_scope_sha256", &self.host_scope_sha256)
            .field("session_issuance_sha256", &self.session_issuance_sha256)
            .field("reader_provenance_sha256", &self.reader_provenance_sha256)
            .field("raw_observation_sha256", &self.raw_observation_sha256)
            .field("layers", &self.layers)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct RawFrame {
    marketplace: Option<Vec<u8>>,
    cache: Option<Vec<u8>>,
    registry: Option<Vec<u8>>,
    plugins_ui: Option<Vec<u8>>,
    runtime: Option<RuntimeObservation>,
}

pub(super) fn capture_host_observations(
    transaction: &mut dyn HostSurfaceTransaction,
    marketplace_plan: &MarketplacePlan,
    binding: &JourneyBinding,
    host_scope: &Arc<BoundHostScope>,
    issuance: &Arc<SessionIssuance>,
    teardown: bool,
) -> Result<HostObservationFrame, HostLifecycleError> {
    let reader_provenance_sha256 = transaction.provenance_sha256().to_owned();
    if reader_provenance_sha256 != binding.binding_sha256() {
        return Err(HostLifecycleError::new(
            HostLifecycleErrorId::IdentityMismatch,
        ));
    }
    let expected = HostObservationExpectations {
        marketplace_plan,
        binding,
        host_scope,
        issuance,
        teardown,
    };
    host_scope.revalidate()?;
    issuance.require_observation_ready()?;
    let before = read_raw(transaction)?;
    host_scope.revalidate()?;
    issuance.require_observation_ready()?;
    if transaction.provenance_sha256() != reader_provenance_sha256 {
        return Err(HostLifecycleError::new(
            HostLifecycleErrorId::ObservationChanged,
        ));
    }
    let after = read_raw(transaction)?;
    if before != after {
        return Err(HostLifecycleError::new(
            HostLifecycleErrorId::ObservationChanged,
        ));
    }
    host_scope.revalidate()?;
    issuance.require_observation_ready()?;
    if transaction.provenance_sha256() != reader_provenance_sha256 {
        return Err(HostLifecycleError::new(
            HostLifecycleErrorId::ObservationChanged,
        ));
    }
    validate_frame(before, &expected, reader_provenance_sha256)
}

fn read_raw(transaction: &mut dyn HostSurfaceTransaction) -> Result<RawFrame, HostLifecycleError> {
    Ok(RawFrame {
        marketplace: transaction
            .read_marketplace(OBSERVATION_LIMIT)
            .map_err(|_| unavailable())?,
        cache: transaction
            .read_cache(OBSERVATION_LIMIT)
            .map_err(|_| unavailable())?,
        registry: transaction
            .read_registry(OBSERVATION_LIMIT)
            .map_err(|_| unavailable())?,
        plugins_ui: transaction
            .read_plugins_ui(OBSERVATION_LIMIT)
            .map_err(|_| unavailable())?,
        runtime: transaction.read_runtime().map_err(|_| unavailable())?,
    })
}
