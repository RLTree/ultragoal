use crate::context::ReadSession;
use crate::inventory::InventoryError;
use serde::Deserialize;
use std::path::Path;

const MAX_AGENT_BYTES: u64 = 64 * 1024;
const MAX_DESCRIPTION_BYTES: usize = 1024;
const MAX_INSTRUCTIONS_BYTES: usize = 16 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AgentManifest {
    name: String,
    description: String,
    developer_instructions: String,
    sandbox_mode: String,
}

fn safe_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn safe_text(value: &str, maximum: usize, multiline: bool) -> bool {
    !value.trim().is_empty()
        && value.len() <= maximum
        && !value.chars().any(|character| {
            character.is_control() && !(multiline && matches!(character, '\n' | '\r' | '\t'))
        })
}

pub(crate) fn inspect(
    reads: &ReadSession,
    path: &Path,
    expected_name: &str,
) -> Result<String, InventoryError> {
    if path.extension().and_then(|value| value.to_str()) != Some("toml") {
        return Err(InventoryError::InvalidRegistry(
            "project agent must use a .toml file".to_owned(),
        ));
    }
    let bytes = reads
        .read_bounded(path, MAX_AGENT_BYTES)
        .map_err(|error| InventoryError::Io {
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;
    let text = std::str::from_utf8(&bytes).map_err(|_| {
        InventoryError::InvalidRegistry("project agent manifest is not UTF-8".to_owned())
    })?;
    let manifest: AgentManifest = toml::from_str(text).map_err(|_| {
        InventoryError::InvalidRegistry("project agent manifest is invalid".to_owned())
    })?;
    if !safe_name(&manifest.name)
        || manifest.name != expected_name
        || !safe_text(&manifest.description, MAX_DESCRIPTION_BYTES, false)
        || !safe_text(
            &manifest.developer_instructions,
            MAX_INSTRUCTIONS_BYTES,
            true,
        )
        || manifest.sandbox_mode != "read-only"
    {
        return Err(InventoryError::InvalidRegistry(
            "project agent manifest violates the read-only role contract".to_owned(),
        ));
    }
    Ok(manifest.name)
}
