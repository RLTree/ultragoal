fn ordered_layers(
    package: HostLayerReport,
    installed: HostLayerReport,
    observed: &[HostLayerReport],
) -> Result<Vec<HostLayerReport>, HostLifecycleError> {
    let mut rows = vec![package];
    for layer in HostLayer::ALL.into_iter().skip(1) {
        if layer == HostLayer::Installed {
            rows.push(installed.clone());
            continue;
        }
        let row = observed
            .iter()
            .find(|row| row.layer() == layer)
            .ok_or_else(|| HostLifecycleError::new(HostLifecycleErrorId::IdentityMismatch))?;
        rows.push(row.clone());
    }
    if rows.len() != HostLayer::ALL.len() {
        return Err(HostLifecycleError::new(
            HostLifecycleErrorId::IdentityMismatch,
        ));
    }
    Ok(rows)
}

fn identity_chain(
    package: &PackageSnapshot,
    binding: &JourneyBinding,
    installed_sha256: &str,
    frame: &HostObservationFrame,
) -> Result<Option<String>, HostLifecycleError> {
    let Some(marketplace) = frame.marketplace.as_ref() else {
        return Ok(None);
    };
    let Some(cache) = frame.cache.as_ref() else {
        return Ok(None);
    };
    let Some(discovery) = frame.discovery.as_ref() else {
        return Ok(None);
    };
    let Some(runtime) = frame.runtime.as_ref() else {
        return Ok(None);
    };
    let Some(marketplace_sha256) = marketplace.catalog_sha256() else {
        return Ok(None);
    };
    let Some(discovery_sha256) = discovery.observation_sha256() else {
        return Ok(None);
    };
    let Some(runtime_sha256) = runtime.output_sha256() else {
        return Ok(None);
    };
    verify_host_identity_chain(
        package.identity(),
        binding,
        marketplace_sha256,
        installed_sha256,
        cache.observation_sha256(),
        discovery_sha256,
        runtime_sha256,
    )
    .map(Some)
}

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn invalid() -> HostLifecycleError {
    HostLifecycleError::new(HostLifecycleErrorId::InvalidBinding)
}

fn transaction_error(error: HostSurfaceTransactionError) -> HostLifecycleError {
    HostLifecycleError::new(match error {
        HostSurfaceTransactionError::Unsupported => {
            HostLifecycleErrorId::ObservationTransactionUnsupported
        }
        HostSurfaceTransactionError::Failed => HostLifecycleErrorId::ObservationUnavailable,
    })
}
