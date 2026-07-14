mod contracts;
mod parser;
mod path;
mod shard;
mod validation;

#[cfg(test)]
mod tests;

pub(crate) use contracts::{
    GeneratedAuthorityParseRequest, GeneratedAuthorityRegistry, GeneratedSurface, RepositoryPath,
    Sha256Digest,
};
pub(crate) use parser::parse;
pub(crate) use shard::{
    GeneratedAuthorityShardParseRequest, GeneratedSurfaceDefinition, parse_shard,
};

pub(crate) const REGISTRY_PATH: &str = "migration/generated-surface-authority.json";
