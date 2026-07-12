use crate::distribution::error::{DistributionError, DistributionErrorId, error};
use crate::distribution::json;
use crate::distribution::model::SurfaceIdentity;
use crate::distribution::model::{Capability, Layer};
use crate::plugin_manifest::Version;
use serde::Deserialize;
use std::collections::BTreeMap;

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
    pub(crate) capabilities: BTreeMap<Capability, CapabilityState>,
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
        capabilities,
        layers,
    })
}

fn validate_layer(
    row: &RawLayer,
    capabilities: &BTreeMap<Capability, CapabilityState>,
) -> Result<(), DistributionError> {
    match row.state {
        ObservationState::Observed => {
            if row.identity_path.is_none()
                || row.payload_path.is_some() != row.layer.requires_payload()
                || row.capability.is_some()
                || row.reason_id.is_some()
                || row.expected.is_some()
            {
                return Err(error(DistributionErrorId::InvalidSpec));
            }
            if capabilities.get(&row.layer.capability()) != Some(&CapabilityState::Supported) {
                return Err(error(DistributionErrorId::CapabilityMismatch));
            }
        }
        ObservationState::DefinitionOnly => {
            let Some(expected) = row.expected.as_ref() else {
                return Err(error(DistributionErrorId::InvalidSpec));
            };
            if row.identity_path.is_some()
                || row.payload_path.is_some()
                || row.capability.is_some()
                || row.reason_id.is_some()
                || expected.plugin_id != "harness-ultragoal"
                || expected.plugin_id.is_empty()
                || Version::parse(&expected.version).is_none()
                || expected
                    .artifact_sha256
                    .as_deref()
                    .is_some_and(|value| !digest(value))
            {
                return Err(error(DistributionErrorId::InvalidSpec));
            }
        }
        ObservationState::Missing => {
            if row.identity_path.is_some()
                || row.payload_path.is_some()
                || row.capability.is_some()
                || row.reason_id.is_some()
                || row.expected.is_some()
            {
                return Err(error(DistributionErrorId::InvalidSpec));
            }
        }
        ObservationState::Unavailable => {
            if row.capability != Some(row.layer.capability())
                || capabilities.get(&row.layer.capability()) != Some(&CapabilityState::Unavailable)
                || row
                    .reason_id
                    .as_deref()
                    .is_none_or(|value| !reason_id(value))
                || row.identity_path.is_some()
                || row.payload_path.is_some()
                || row.expected.is_some()
            {
                return Err(error(DistributionErrorId::CapabilityMismatch));
            }
        }
    }
    Ok(())
}

pub(crate) fn parse_envelope(bytes: &[u8]) -> Result<Envelope, DistributionError> {
    let envelope: Envelope = json::parse(bytes, IDENTITY_LIMIT)?;
    if envelope.schema != "harness-ultragoal.distribution-observation.v1"
        || envelope.plugin_id.is_empty()
        || envelope.plugin_id.len() > 128
        || Version::parse(&envelope.version).is_none()
        || envelope
            .artifact_sha256
            .as_deref()
            .is_some_and(|value| !digest(value))
        || !matches!(
            envelope.exposure.as_str(),
            "affirmed" | "denied" | "not-applicable"
        )
    {
        return Err(error(DistributionErrorId::InvalidSpec));
    }
    Ok(envelope)
}

pub(crate) fn digest(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|hex| {
        hex.len() == 64
            && hex
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    })
}

fn reason_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 96
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawIdentityLadder {
    schema: String,
    surfaces: Vec<SurfaceIdentity>,
}

pub(crate) fn parse_identity_ladder(
    bytes: &[u8],
) -> Result<Vec<SurfaceIdentity>, DistributionError> {
    let value: RawIdentityLadder = json::parse(bytes, REQUEST_LIMIT)?;
    if value.schema != "harness-ultragoal.distribution-identity-ladder.v1" {
        return Err(error(DistributionErrorId::InvalidSpec));
    }
    Ok(value.surfaces)
}
