use super::model::DependencyRegistry;
use std::path::Path;

pub(super) fn load(path: &Path) -> Result<DependencyRegistry, String> {
    let bytes = crate::digest::read_file_bytes(path)
        .map_err(|error| format!("dependency_adapter_registry_unreadable:{error}"))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("dependency_adapter_registry_invalid:{error}"))
}
