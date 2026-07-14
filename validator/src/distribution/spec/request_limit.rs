pub(crate) const REQUEST_LIMIT: usize = 1024 * 1024;
pub(crate) const IDENTITY_LIMIT: usize = 64 * 1024;
pub(crate) const PAYLOAD_LIMIT: usize = 4 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum CapabilityState {
    Supported,
    Unavailable,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ObservationState {
    Observed,
    DefinitionOnly,
    Missing,
    Unavailable,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRequest {
    schema: String,
    context_id: String,
    candidate_id: String,
    host: RawHost,
    layers: Vec<RawLayer>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawHost {
    platform: String,
    version: String,
    capabilities: Vec<RawCapability>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCapability {
    capability: Capability,
    state: CapabilityState,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawLayer {
    layer: Layer,
    state: ObservationState,
    #[serde(default)]
    identity_path: Option<String>,
    #[serde(default)]
    payload_path: Option<String>,
    #[serde(default)]
    capability: Option<Capability>,
    #[serde(default)]
    reason_id: Option<String>,
    #[serde(default)]
    expected: Option<ExpectedIdentity>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExpectedIdentity {
    pub(crate) plugin_id: String,
    pub(crate) version: String,
    pub(crate) artifact_sha256: Option<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct Request {
    pub(crate) context_id: String,
    pub(crate) candidate_id: String,
    pub(crate) platform: String,
    pub(crate) layers: BTreeMap<Layer, LayerSpec>,
}

#[derive(Clone, Debug)]
pub(crate) struct LayerSpec {
    pub(crate) state: ObservationState,
    pub(crate) identity_path: Option<String>,
    pub(crate) payload_path: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Envelope {
    pub(crate) schema: String,
    pub(crate) layer: Layer,
    pub(crate) context_id: String,
    pub(crate) candidate_id: String,
    pub(crate) plugin_id: String,
    pub(crate) version: String,
    pub(crate) artifact_sha256: Option<String>,
    pub(crate) exposure: String,
}

pub(crate) fn parse_request(bytes: &[u8]) -> Result<Request, DistributionError> {
    let raw: RawRequest = json::parse(bytes, REQUEST_LIMIT)?;
    if raw.schema != "harness-ultragoal.distribution-request.v1"
        || !digest(&raw.context_id)
        || !digest(&raw.candidate_id)
        || raw.host.version.is_empty()
        || raw.host.version.len() > 128
    {
        return Err(error(DistributionErrorId::InvalidSpec));
    }
    let mut capabilities = BTreeMap::new();
    for row in raw.host.capabilities {
        if capabilities.insert(row.capability, row.state).is_some() {
            return Err(error(DistributionErrorId::DuplicateCapability));
        }
    }
    if capabilities.len() != Capability::ALL.len()
        || Capability::ALL
            .iter()
            .any(|item| !capabilities.contains_key(item))
    {
        return Err(error(DistributionErrorId::InvalidSpec));
    }
    let mut layers = BTreeMap::new();
    for row in raw.layers {
        if layers.contains_key(&row.layer) {
            return Err(error(DistributionErrorId::DuplicateLayer));
        }
        validate_layer(&row, &capabilities)?;
        layers.insert(
            row.layer,
            LayerSpec {
                state: row.state,
                identity_path: row.identity_path,
                payload_path: row.payload_path,
            },
        );
    }
    if layers.len() != Layer::ALL.len() || Layer::ALL.iter().any(|item| !layers.contains_key(item))
    {
        return Err(error(DistributionErrorId::InvalidSpec));
    }
    if !matches!(
        raw.host.platform.as_str(),
        "macos" | "linux" | "windows" | "other"
    ) {
        return Err(error(DistributionErrorId::InvalidSpec));
    }
    Ok(Request {
        context_id: raw.context_id,
        candidate_id: raw.candidate_id,
        platform: raw.host.platform,
        layers,
    })
}
