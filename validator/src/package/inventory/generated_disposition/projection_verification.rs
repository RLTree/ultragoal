use super::{MAX_OUTPUT_BYTES, Source, adopted_schema_contract, digest_hex};
use crate::generated_authority::{GeneratedAuthorityRegistry, GeneratedSurface};

pub(super) fn verify(
    source: &mut impl Source,
    parsed: &GeneratedAuthorityRegistry,
) -> Result<(), String> {
    source
        .read(
            parsed.registry_projection.generator.as_str(),
            MAX_OUTPUT_BYTES,
        )
        .map_err(|error| {
            format!("generated registry projection generator is unavailable: {error}")
        })?;
    for canonical_source in &parsed.registry_projection.canonical_sources {
        source
            .read(canonical_source.as_str(), MAX_OUTPUT_BYTES)
            .map_err(|error| {
                format!("generated registry projection canonical source is unavailable: {error}")
            })?;
    }
    for (output, row) in &parsed.surfaces {
        let (generator, canonical_sources, output_sha256) = match row {
            GeneratedSurface::AdoptedSchemaContract {
                output,
                sha256,
                schema,
                schema_sha256,
                source_contract,
                source_contract_sha256,
                amendment_log,
                amendment_id,
                amendment_hash,
            } => {
                adopted_schema_contract::verify(
                    source,
                    adopted_schema_contract::Binding {
                        output,
                        sha256,
                        schema,
                        schema_sha256,
                        source_contract,
                        source_contract_sha256,
                        amendment_log,
                        amendment_id,
                        amendment_hash,
                    },
                )?;
                continue;
            }
            GeneratedSurface::SourceProjection {
                generator,
                canonical_sources,
                output_sha256,
                ..
            } => (Some(generator), canonical_sources, output_sha256),
            GeneratedSurface::ToolProjection {
                canonical_sources,
                output_sha256,
                ..
            } => (None, canonical_sources, output_sha256),
            GeneratedSurface::CanonicalProjection { .. } => {
                return Err(
                    "canonical projection cannot authorize current generated authority".to_string(),
                );
            }
            GeneratedSurface::RetainedContext { .. } => continue,
        };
        if let Some(generator) = generator {
            source
                .read(generator.as_str(), MAX_OUTPUT_BYTES)
                .map_err(|error| format!("source projection generator is unavailable: {error}"))?;
        }
        for canonical_source in canonical_sources {
            source
                .read(canonical_source.as_str(), MAX_OUTPUT_BYTES)
                .map_err(|error| format!("projection canonical source is unavailable: {error}"))?;
        }
        let bytes = source
            .read(output.as_str(), MAX_OUTPUT_BYTES)
            .map_err(|error| format!("projection output is unavailable: {error}"))?;
        if digest_hex(bytes.as_ref()) != output_sha256.lowercase_hex() {
            return Err("projection output digest mismatch".to_string());
        }
    }
    Ok(())
}
