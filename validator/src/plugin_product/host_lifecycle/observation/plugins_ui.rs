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
