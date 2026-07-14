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
