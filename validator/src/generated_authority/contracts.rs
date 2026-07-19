use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct RepositoryPath(String);

impl RepositoryPath {
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    pub(super) fn validated(value: String) -> Self {
        Self(value)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Sha256Digest([u8; 32]);

impl Sha256Digest {
    pub(crate) fn lowercase_hex(&self) -> String {
        self.0.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    pub(super) fn validated(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RegistryProjection {
    pub(crate) generator: RepositoryPath,
    pub(crate) canonical_sources: Vec<RepositoryPath>,
    pub(crate) regeneration_command: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum GeneratedSurface {
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
        output_sha256: Sha256Digest,
    },
    ToolProjection {
        output: RepositoryPath,
        tool: String,
        tool_version: String,
        canonical_sources: Vec<RepositoryPath>,
        regeneration_command: String,
        output_sha256: Sha256Digest,
    },
}

impl GeneratedSurface {
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GeneratedAuthorityRegistry {
    pub(crate) registry_projection: RegistryProjection,
    pub(crate) surfaces: BTreeMap<RepositoryPath, GeneratedSurface>,
}

pub(crate) struct GeneratedAuthorityParseRequest<'a> {
    pub(crate) bytes: &'a [u8],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GeneratedAuthorityParseResponse {
    pub(crate) registry: GeneratedAuthorityRegistry,
    pub(crate) canonical_bytes: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GeneratedAuthorityParseError {
    pub(crate) code: &'static str,
}

impl GeneratedAuthorityParseError {
    pub(crate) fn stable_text(&self) -> &'static str {
        self.code
    }
}
