use super::super::contracts::{RepositoryPath, Sha256Digest};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum GeneratedSurfaceDefinition {
    AdoptedSchemaContract {
        output: RepositoryPath,
        sha256: Sha256Digest,
        schema: RepositoryPath,
        source_contract: RepositoryPath,
        source_contract_sha256: Sha256Digest,
        amendment_log: RepositoryPath,
        amendment_id: String,
        amendment_hash: Sha256Digest,
    },
    CanonicalProjection {
        output: RepositoryPath,
        generator: String,
        inputs: Vec<RepositoryPath>,
    },
    RetainedContext {
        output: RepositoryPath,
        sha256: Sha256Digest,
        reason: String,
        replacement_targets: Vec<String>,
    },
    SourceProjection {
        output: RepositoryPath,
        generator: RepositoryPath,
        canonical_sources: Vec<RepositoryPath>,
        regeneration_command: String,
    },
    ToolProjection {
        output: RepositoryPath,
        tool: String,
        tool_version: String,
        canonical_sources: Vec<RepositoryPath>,
        regeneration_command: String,
    },
}

impl GeneratedSurfaceDefinition {
    pub(crate) fn output(&self) -> &RepositoryPath {
        match self {
            Self::AdoptedSchemaContract { output, .. }
            | Self::CanonicalProjection { output, .. }
            | Self::RetainedContext { output, .. }
            | Self::SourceProjection { output, .. }
            | Self::ToolProjection { output, .. } => output,
        }
    }
}

pub(crate) struct GeneratedAuthorityShardParseRequest<'a> {
    pub(crate) bytes: &'a [u8],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GeneratedAuthorityShardParseResponse {
    pub(crate) definitions: BTreeMap<RepositoryPath, GeneratedSurfaceDefinition>,
    pub(crate) canonical_bytes: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GeneratedAuthorityShardParseError {
    pub(crate) code: &'static str,
}

impl GeneratedAuthorityShardParseError {
    pub(crate) fn stable_text(&self) -> &'static str {
        self.code
    }
}
