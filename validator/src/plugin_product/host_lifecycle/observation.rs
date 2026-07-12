use super::error::{HostLifecycleError, HostLifecycleErrorId};
use super::issuance::{SessionIssuance, same_issuance};
use super::model::{HostLayer, HostLayerReport, HostLayerVerdict, PluginsUiObservation};
use super::scope::{BoundHostScope, same_scope};
use crate::distribution::{
    AppRegistryObservation, AppRegistryVerdict, CacheSnapshot, Capability, DiscoveryObservation,
    DiscoveryVerdict, HostCapabilityState, JourneyBinding, MarketplacePlan, MarketplaceSnapshot,
    RuntimeObservation, observe_app_registry, observe_codex_marketplace, observe_discovery,
    reconcile_cache_read_only,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fmt;
use std::sync::Arc;

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

fn validate_frame(
    raw: RawFrame,
    expected: &HostObservationExpectations<'_>,
    reader_provenance_sha256: String,
) -> Result<HostObservationFrame, HostLifecycleError> {
    let raw_observation_sha256 = digest(
        &serde_json::to_vec(&raw)
            .map_err(|_| HostLifecycleError::new(HostLifecycleErrorId::IdentityMismatch))?,
    );
    let (marketplace, marketplace_row) = marketplace(&raw, expected)?;
    let (cache, cache_row) = cache(&raw, expected)?;
    let (app_registry, app_row, discovery, discovery_row) = registry(&raw, expected)?;
    let (plugins_ui, ui_row) = plugins_ui(&raw, expected)?;
    let (runtime, runtime_row) = runtime(&raw, expected)?;
    expected.host_scope.revalidate()?;
    expected.issuance.require_observation_ready()?;
    Ok(HostObservationFrame {
        binding_sha256: expected.binding.binding_sha256().to_owned(),
        host_scope: Arc::clone(expected.host_scope),
        host_scope_sha256: expected.host_scope.scope_sha256().to_owned(),
        session_issuance: Arc::clone(expected.issuance),
        session_issuance_sha256: expected.issuance.issuance_sha256().to_owned(),
        reader_provenance_sha256,
        raw_observation_sha256,
        layers: vec![
            marketplace_row,
            cache_row,
            app_row,
            ui_row,
            discovery_row,
            runtime_row,
        ],
        marketplace,
        cache,
        app_registry,
        plugins_ui,
        discovery,
        runtime,
    })
}

fn marketplace(
    raw: &RawFrame,
    expected: &HostObservationExpectations<'_>,
) -> Result<(Option<MarketplaceSnapshot>, HostLayerReport), HostLifecycleError> {
    match expected.host_scope.host().state(Capability::Marketplace) {
        HostCapabilityState::Supported => {
            let bytes = raw.marketplace.as_deref().ok_or_else(unavailable)?;
            let snapshot = observe_codex_marketplace(
                bytes,
                expected.marketplace_plan,
                expected.host_scope.marketplace_scope(),
            )
            .map_err(|_| conflict())?;
            let digest = snapshot.catalog_sha256().map(str::to_owned);
            Ok((
                Some(snapshot),
                report(HostLayer::Marketplace, HostLayerVerdict::Verified, digest),
            ))
        }
        HostCapabilityState::Unsupported => {
            reject_supplied(&raw.marketplace)?;
            Ok((
                None,
                report(HostLayer::Marketplace, HostLayerVerdict::Unsupported, None),
            ))
        }
        HostCapabilityState::Absent => {
            reject_supplied(&raw.marketplace)?;
            Ok((
                None,
                report(
                    HostLayer::Marketplace,
                    HostLayerVerdict::CapabilityAbsent,
                    None,
                ),
            ))
        }
    }
}

fn cache(
    raw: &RawFrame,
    expected: &HostObservationExpectations<'_>,
) -> Result<(Option<CacheSnapshot>, HostLayerReport), HostLifecycleError> {
    match expected.host_scope.host().state(Capability::Cache) {
        HostCapabilityState::Supported if expected.teardown && raw.cache.is_none() => Ok((
            None,
            report(HostLayer::Cache, HostLayerVerdict::ObservedAbsent, None),
        )),
        HostCapabilityState::Supported => {
            let bytes = raw.cache.as_deref().ok_or_else(unavailable)?;
            let snapshot = reconcile_cache_read_only(bytes, expected.host_scope.cache())
                .map_err(|_| conflict())?;
            let digest = snapshot.observation_sha256().to_owned();
            Ok((
                Some(snapshot),
                report(HostLayer::Cache, HostLayerVerdict::Verified, Some(digest)),
            ))
        }
        HostCapabilityState::Unsupported => {
            reject_supplied(&raw.cache)?;
            Ok((
                None,
                report(HostLayer::Cache, HostLayerVerdict::Unsupported, None),
            ))
        }
        HostCapabilityState::Absent => {
            reject_supplied(&raw.cache)?;
            Ok((
                None,
                report(HostLayer::Cache, HostLayerVerdict::CapabilityAbsent, None),
            ))
        }
    }
}

#[allow(clippy::type_complexity)]
fn registry(
    raw: &RawFrame,
    expected: &HostObservationExpectations<'_>,
) -> Result<
    (
        Option<AppRegistryObservation>,
        HostLayerReport,
        Option<DiscoveryObservation>,
        HostLayerReport,
    ),
    HostLifecycleError,
> {
    if expected.teardown
        && raw.registry.is_none()
        && expected.host_scope.host().state(Capability::AppRegistry)
            == HostCapabilityState::Supported
        && expected.host_scope.host().state(Capability::Discovery) == HostCapabilityState::Supported
    {
        return Ok((
            None,
            report(
                HostLayer::AppRegistry,
                HostLayerVerdict::ObservedAbsent,
                None,
            ),
            None,
            report(HostLayer::Discovery, HostLayerVerdict::ObservedAbsent, None),
        ));
    }
    let app = observe_app_registry(
        raw.registry.as_deref(),
        expected.binding,
        expected.host_scope.host(),
    )
    .map_err(|_| conflict())?;
    let discovery = observe_discovery(
        raw.registry.as_deref(),
        expected.binding,
        expected.host_scope.host(),
    )
    .map_err(|_| conflict())?;
    let app_verdict = match app.verdict() {
        AppRegistryVerdict::Verified => HostLayerVerdict::Verified,
        AppRegistryVerdict::Unsupported => HostLayerVerdict::Unsupported,
        AppRegistryVerdict::Absent => HostLayerVerdict::CapabilityAbsent,
    };
    let discovery_verdict = match discovery.discovery_verdict() {
        DiscoveryVerdict::Visible => HostLayerVerdict::Verified,
        DiscoveryVerdict::Unsupported => HostLayerVerdict::Unsupported,
        DiscoveryVerdict::Absent => HostLayerVerdict::CapabilityAbsent,
        DiscoveryVerdict::RegisteredHidden | DiscoveryVerdict::DefinitionOnly => {
            return Err(conflict());
        }
    };
    let app_digest = app.observation_sha256().map(str::to_owned);
    let discovery_digest = discovery.observation_sha256().map(str::to_owned);
    Ok((
        Some(app),
        report(HostLayer::AppRegistry, app_verdict, app_digest),
        Some(discovery),
        report(HostLayer::Discovery, discovery_verdict, discovery_digest),
    ))
}

fn plugins_ui(
    raw: &RawFrame,
    expected: &HostObservationExpectations<'_>,
) -> Result<(PluginsUiObservation, HostLayerReport), HostLifecycleError> {
    let observation = match expected.host_scope.host().state(Capability::PluginsUi) {
        HostCapabilityState::Supported => observe_plugins_ui(
            raw.plugins_ui.as_deref().ok_or_else(unavailable)?,
            expected.binding,
        )?,
        HostCapabilityState::Unsupported => {
            reject_supplied(&raw.plugins_ui)?;
            PluginsUiObservation::new(
                HostLayerVerdict::Unsupported,
                None,
                expected.binding.binding_sha256().to_owned(),
            )
        }
        HostCapabilityState::Absent => {
            reject_supplied(&raw.plugins_ui)?;
            PluginsUiObservation::new(
                HostLayerVerdict::CapabilityAbsent,
                None,
                expected.binding.binding_sha256().to_owned(),
            )
        }
    };
    let row = report(
        HostLayer::PluginsUi,
        observation.verdict(),
        observation.observation_sha256().map(str::to_owned),
    );
    Ok((observation, row))
}

fn runtime(
    raw: &RawFrame,
    expected: &HostObservationExpectations<'_>,
) -> Result<(Option<RuntimeObservation>, HostLayerReport), HostLifecycleError> {
    match expected.host_scope.host().state(Capability::Runtime) {
        HostCapabilityState::Supported if expected.teardown && raw.runtime.is_none() => Ok((
            None,
            report(HostLayer::Runtime, HostLayerVerdict::ObservedAbsent, None),
        )),
        HostCapabilityState::Supported => {
            let observation = raw.runtime.clone().ok_or_else(unavailable)?;
            if !observation.is_current_execution() {
                return Err(conflict());
            }
            let digest = observation.output_sha256().map(str::to_owned);
            Ok((
                Some(observation),
                report(HostLayer::Runtime, HostLayerVerdict::Verified, digest),
            ))
        }
        HostCapabilityState::Unsupported => {
            reject_supplied(&raw.runtime)?;
            Ok((
                None,
                report(HostLayer::Runtime, HostLayerVerdict::Unsupported, None),
            ))
        }
        HostCapabilityState::Absent => {
            reject_supplied(&raw.runtime)?;
            Ok((
                None,
                report(HostLayer::Runtime, HostLayerVerdict::CapabilityAbsent, None),
            ))
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UiDocument {
    schema: String,
    context_id: String,
    candidate_id: String,
    home_id: String,
    project_id: String,
    host_id: String,
    capability_sha256: String,
    binding_sha256: String,
    entries: Vec<UiEntry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UiEntry {
    plugin_id: String,
    version: String,
    package_sha256: String,
    installed_tree_sha256: String,
    visible: bool,
}

pub fn observe_plugins_ui(
    bytes: &[u8],
    binding: &JourneyBinding,
) -> Result<PluginsUiObservation, HostLifecycleError> {
    if bytes.len() > OBSERVATION_LIMIT {
        return Err(conflict());
    }
    let document: UiDocument = serde_json::from_slice(bytes).map_err(|_| conflict())?;
    let source = binding.package().source();
    if document.schema != "harness-ultragoal.plugins-ui-observation.v1"
        || document.context_id != source.context_id()
        || document.candidate_id != source.candidate_id()
        || document.home_id != binding.home_id()
        || document.project_id != binding.project_id()
        || document.host_id != binding.host_id()
        || document.capability_sha256 != binding.capability_sha256()
        || document.binding_sha256 != binding.binding_sha256()
        || document.entries.is_empty()
        || document.entries.len() > 1024
    {
        return Err(conflict());
    }
    let mut seen = BTreeSet::new();
    let matching = document
        .entries
        .iter()
        .filter(|row| {
            seen.insert((row.plugin_id.as_str(), row.version.as_str()))
                && row.plugin_id == source.plugin_id()
        })
        .collect::<Vec<_>>();
    if seen.len() != document.entries.len()
        || matching.len() != 1
        || matching[0].version != source.version()
        || matching[0].package_sha256 != binding.package().archive_sha256()
        || matching[0].installed_tree_sha256 != binding.package().tree_sha256()
        || !matching[0].visible
    {
        return Err(conflict());
    }
    Ok(PluginsUiObservation::new(
        HostLayerVerdict::Verified,
        Some(digest(bytes)),
        binding.binding_sha256().to_owned(),
    ))
}

fn report(layer: HostLayer, verdict: HostLayerVerdict, digest: Option<String>) -> HostLayerReport {
    HostLayerReport::new(layer, verdict, digest)
}

fn reject_supplied<T>(value: &Option<T>) -> Result<(), HostLifecycleError> {
    if value.is_some() {
        return Err(HostLifecycleError::new(
            HostLifecycleErrorId::UnsupportedSubstitution,
        ));
    }
    Ok(())
}

fn unavailable() -> HostLifecycleError {
    HostLifecycleError::new(HostLifecycleErrorId::ObservationUnavailable)
}

fn conflict() -> HostLifecycleError {
    HostLifecycleError::new(HostLifecycleErrorId::ObservationConflict)
}

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
