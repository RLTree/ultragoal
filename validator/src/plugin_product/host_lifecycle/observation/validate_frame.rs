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
    let registry = registry(&raw, expected)?;
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
            registry.app_report,
            ui_row,
            registry.discovery_report,
            runtime_row,
        ],
        marketplace,
        cache,
        app_registry: registry.app_registry,
        plugins_ui,
        discovery: registry.discovery,
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

struct RegistryLayers {
    app_registry: Option<AppRegistryObservation>,
    app_report: HostLayerReport,
    discovery: Option<DiscoveryObservation>,
    discovery_report: HostLayerReport,
}

fn registry(
    raw: &RawFrame,
    expected: &HostObservationExpectations<'_>,
) -> Result<RegistryLayers, HostLifecycleError> {
    if expected.teardown
        && raw.registry.is_none()
        && expected.host_scope.host().state(Capability::AppRegistry)
            == HostCapabilityState::Supported
        && expected.host_scope.host().state(Capability::Discovery) == HostCapabilityState::Supported
    {
        return Ok(RegistryLayers {
            app_registry: None,
            app_report: report(
                HostLayer::AppRegistry,
                HostLayerVerdict::ObservedAbsent,
                None,
            ),
            discovery: None,
            discovery_report: report(HostLayer::Discovery, HostLayerVerdict::ObservedAbsent, None),
        });
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
    Ok(RegistryLayers {
        app_registry: Some(app),
        app_report: report(HostLayer::AppRegistry, app_verdict, app_digest),
        discovery: Some(discovery),
        discovery_report: report(HostLayer::Discovery, discovery_verdict, discovery_digest),
    })
}
