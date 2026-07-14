use super::model::EnforcementRegistry;

pub(crate) struct StandardsRegistryParseRequest<'a> {
    pub(crate) bytes: &'a [u8],
}

pub(crate) struct StandardsRegistryParseResponse {
    pub(crate) registry: EnforcementRegistry,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum StandardsRegistryParseError {
    InvalidRegistry,
}

pub(crate) fn parse(
    request: StandardsRegistryParseRequest<'_>,
) -> Result<StandardsRegistryParseResponse, StandardsRegistryParseError> {
    serde_json::from_slice(request.bytes)
        .map(|registry| StandardsRegistryParseResponse { registry })
        .map_err(|_| StandardsRegistryParseError::InvalidRegistry)
}
