use super::{descriptor_parser, invalid_source, too_large};
use crate::plugin_product::agent_discovery::error::AgentDiscoveryError;
use crate::plugin_product::agent_discovery::model::{MAX_DESCRIPTOR_BYTES, ProjectAgentDescriptor};

pub(crate) struct GlobalAgentDescriptor {
    pub(crate) name: String,
    pub(crate) sandbox_mode: Option<String>,
}

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

pub(crate) fn parse_global_descriptor(
    bytes: &[u8],
) -> Result<GlobalAgentDescriptor, AgentDiscoveryError> {
    if bytes.len() > MAX_DESCRIPTOR_BYTES {
        return Err(too_large());
    }
    let text = std::str::from_utf8(bytes).map_err(|_| invalid_source())?;
    let value: toml::Value = toml::from_str(text).map_err(|_| invalid_source())?;
    let table = value.as_table().ok_or_else(invalid_source)?;
    let name = table
        .get("name")
        .and_then(toml::Value::as_str)
        .filter(|name| safe_name(name))
        .ok_or_else(invalid_source)?
        .to_owned();
    let sandbox_mode = match table.get("sandbox_mode") {
        Some(value) => Some(
            value
                .as_str()
                .filter(|value| safe_text(value, 80, false))
                .ok_or_else(invalid_source)?
                .to_owned(),
        ),
        None => None,
    };
    Ok(GlobalAgentDescriptor { name, sandbox_mode })
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
