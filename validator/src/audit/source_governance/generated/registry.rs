use crate::generated_authority::{
    GeneratedAuthorityParseRequest, GeneratedAuthorityRegistry, parse,
};
use std::path::Path;

use super::super::GovernedSource;

pub(super) struct LoadedRegistry<'a> {
    pub(super) registry: GeneratedAuthorityRegistry,
    pub(super) source: &'a GovernedSource,
}

pub(super) fn load(sources: &[GovernedSource]) -> Result<LoadedRegistry<'_>, String> {
    let source = sources
        .iter()
        .find(|source| source.relative == crate::generated_authority::REGISTRY_PATH)
        .ok_or_else(|| "generated_source_registry_source_missing".to_string())?;
    let response = parse(GeneratedAuthorityParseRequest {
        bytes: &source.bytes,
    })
    .map_err(|error| format!("generated_source_registry_invalid:{}", error.stable_text()))?;
    if response.canonical_bytes != source.bytes {
        return Err("generated_source_registry_noncanonical".to_string());
    }
    Ok(LoadedRegistry {
        registry: response.registry,
        source,
    })
}

pub(super) fn revalidate(root: &Path, source: &GovernedSource) -> Result<(), String> {
    match source.revalidate(root) {
        Ok(()) => Ok(()),
        Err(error) if error.content_changed() => {
            Err("generated_source_registry_changed_during_session".to_string())
        }
        Err(error) => Err(format!(
            "generated_source_registry_revalidation_failed:{}",
            error.stable_text()
        )),
    }
}
