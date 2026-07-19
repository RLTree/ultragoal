use super::contracts::{
    GeneratedAuthorityParseError, GeneratedAuthorityParseRequest, GeneratedAuthorityParseResponse,
};
use super::validation;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawRegistry {
    pub(super) schema_version: String,
    pub(super) contract_id: String,
    pub(super) registry_projection: RawRegistryProjection,
    pub(super) surfaces: Vec<RawSurface>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawRegistryProjection {
    pub(super) generator: String,
    pub(super) canonical_sources: Vec<String>,
    pub(super) regeneration_command: String,
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "disposition", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum RawSurface {
    AdoptedSchemaContract {
        output: String,
        sha256: String,
        schema: String,
        schema_sha256: String,
        source_contract: String,
        source_contract_sha256: String,
        amendment_log: String,
        amendment_id: String,
        amendment_hash: String,
        claim_ceiling: String,
    },
    CanonicalProjection {
        output: String,
        generator: String,
        recipe: String,
        inputs: Vec<String>,
    },
    RetainedContext {
        output: String,
        sha256: String,
        reason: String,
        replacement_targets: Vec<String>,
        preserve: bool,
        physical_deletion_authorized: bool,
    },
    SourceProjection {
        output: String,
        generator: String,
        canonical_sources: Vec<String>,
        regeneration_command: String,
        output_sha256: String,
    },
    ToolProjection {
        output: String,
        tool: String,
        tool_version: String,
        canonical_sources: Vec<String>,
        regeneration_command: String,
        output_sha256: String,
    },
}

pub(crate) fn parse(
    request: GeneratedAuthorityParseRequest<'_>,
) -> Result<GeneratedAuthorityParseResponse, GeneratedAuthorityParseError> {
    let mut deserializer = serde_json::Deserializer::from_slice(request.bytes);
    let raw = RawRegistry::deserialize(&mut deserializer)
        .map_err(|_| error("generated_authority_json_invalid"))?;
    deserializer
        .end()
        .map_err(|_| error("generated_authority_trailing_data"))?;
    let canonical_bytes = canonical_bytes(&raw)?;
    validation::validate(raw)
        .map(|registry| GeneratedAuthorityParseResponse {
            registry,
            canonical_bytes,
        })
        .map_err(error)
}

fn canonical_bytes(raw: &RawRegistry) -> Result<Vec<u8>, GeneratedAuthorityParseError> {
    let mut bytes = serde_json::to_vec_pretty(raw)
        .map_err(|_| error("generated_authority_canonical_render_failed"))?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn error(code: &'static str) -> GeneratedAuthorityParseError {
    GeneratedAuthorityParseError { code }
}
