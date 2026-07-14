use super::{descriptor_parser, invalid_source, too_large};
use crate::plugin_product::agent_discovery::error::AgentDiscoveryError;
use crate::plugin_product::agent_discovery::model::{MAX_DESCRIPTOR_BYTES, ProjectAgentDescriptor};

pub(crate) fn parse_descriptor(
    bytes: &[u8],
) -> Result<ProjectAgentDescriptor, AgentDiscoveryError> {
    if bytes.len() > MAX_DESCRIPTOR_BYTES {
        return Err(too_large());
    }
    let text = std::str::from_utf8(bytes).map_err(|_| invalid_source())?;
    let descriptor: ProjectAgentDescriptor =
        descriptor_parser::parse(text).map_err(|_| invalid_source())?;
    if !safe_name(&descriptor.name)
        || !safe_text(&descriptor.description, 1024, false)
        || !safe_text(&descriptor.developer_instructions, 16 * 1024, true)
    {
        return Err(invalid_source());
    }
    Ok(descriptor)
}

fn safe_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 80
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
        })
}

fn safe_text(value: &str, maximum: usize, multiline: bool) -> bool {
    !value.trim().is_empty()
        && value.len() <= maximum
        && !value.chars().any(|character| {
            character.is_control() && !(multiline && matches!(character, '\n' | '\r' | '\t'))
        })
}
