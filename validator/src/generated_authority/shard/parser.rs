use super::contracts::{
    GeneratedAuthorityShardParseError, GeneratedAuthorityShardParseRequest,
    GeneratedAuthorityShardParseResponse,
};
use super::validation;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawShard {
    pub(super) schema_version: String,
    pub(super) contract_id: String,
    pub(super) surfaces: Vec<RawDefinition>,
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "disposition", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum RawDefinition {
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
    },
    ToolProjection {
        output: String,
        tool: String,
        tool_version: String,
        canonical_sources: Vec<String>,
        regeneration_command: String,
    },
}

pub(crate) fn parse_shard(
    request: GeneratedAuthorityShardParseRequest<'_>,
) -> Result<GeneratedAuthorityShardParseResponse, GeneratedAuthorityShardParseError> {
    let mut deserializer = serde_json::Deserializer::from_slice(request.bytes);
    let raw = RawShard::deserialize(&mut deserializer)
        .map_err(|_| error("generated_authority_shard_json_invalid"))?;
    deserializer
        .end()
        .map_err(|_| error("generated_authority_shard_trailing_data"))?;
    let canonical_bytes = canonical_bytes(&raw)?;
    validation::validate(raw)
        .map(|definitions| GeneratedAuthorityShardParseResponse {
            definitions,
            canonical_bytes,
        })
        .map_err(error)
}

fn canonical_bytes(raw: &RawShard) -> Result<Vec<u8>, GeneratedAuthorityShardParseError> {
    let mut bytes = serde_json::to_vec_pretty(raw)
        .map_err(|_| error("generated_authority_shard_canonical_render_failed"))?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn error(code: &'static str) -> GeneratedAuthorityShardParseError {
    GeneratedAuthorityShardParseError { code }
}
