use crate::context::ReadSession;
use crate::generated_authority::{GeneratedAuthorityRegistry, GeneratedSurface};
use crate::inventory::digest::file_identity_regular;
use crate::inventory::types::InventoryError;
use std::path::Path;

pub(super) fn verify(
    reads: &ReadSession,
    root: &Path,
    parsed: &GeneratedAuthorityRegistry,
) -> Result<(), InventoryError> {
    regular_digest(
        reads,
        &root.join(parsed.registry_projection.generator.as_str()),
    )?;
    for source in &parsed.registry_projection.canonical_sources {
        regular_digest(reads, &root.join(source.as_str()))?;
    }
    for surface in parsed.surfaces.values() {
        let (output, generator, sources, expected) = match surface {
            GeneratedSurface::SourceProjection {
                output,
                generator,
                canonical_sources,
                output_sha256,
                ..
            } => (
                output,
                Some(generator.as_str()),
                canonical_sources,
                output_sha256,
            ),
            GeneratedSurface::ToolProjection {
                output,
                canonical_sources,
                output_sha256,
                ..
            } => (output, None, canonical_sources, output_sha256),
            _ => continue,
        };
        if let Some(generator) = generator {
            regular_digest(reads, &root.join(generator))?;
        }
        for source in sources {
            regular_digest(reads, &root.join(source.as_str()))?;
        }
        if regular_digest(reads, &root.join(output.as_str()))? != expected.lowercase_hex() {
            return Err(InventoryError::InvalidRegistry(
                "generated provenance projection digest mismatch".to_owned(),
            ));
        }
    }
    Ok(())
}

fn regular_digest(reads: &ReadSession, path: &Path) -> Result<String, InventoryError> {
    file_identity_regular(reads, path).map(|(digest, _)| digest)
}
