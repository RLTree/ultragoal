use super::contracts::RepositoryPath;
use std::path::{Component, Path};

const MAX_PATH_BYTES: usize = 512;

pub(super) fn parse(value: String) -> Result<RepositoryPath, &'static str> {
    if value.is_empty()
        || value.len() > MAX_PATH_BYTES
        || value.starts_with('/')
        || value.ends_with('/')
        || value.contains("//")
        || value.contains('\\')
        || value.bytes().any(|byte| {
            !(byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'.' | b'_' | b'-'))
        })
        || !Path::new(&value)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
    {
        return Err("generated_authority_path_invalid");
    }
    Ok(RepositoryPath::validated(value))
}

pub(super) fn sorted_unique(values: &[RepositoryPath]) -> bool {
    !values.is_empty() && values.windows(2).all(|pair| pair[0] < pair[1])
}
