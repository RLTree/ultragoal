use super::{FitError, FitErrorId, error};
use serde::Serialize;

const MAX_PATH_BYTES: usize = 240;
const MAX_COMPONENT_BYTES: usize = 100;
const MAX_DEPTH: usize = 16;

#[derive(Clone, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct CanonicalPath(String);

impl CanonicalPath {
    pub fn parse(raw: impl Into<String>) -> Result<Self, FitError> {
        let raw = raw.into();
        if raw.is_empty()
            || raw.len() > MAX_PATH_BYTES
            || !raw.is_ascii()
            || raw.starts_with('/')
            || raw.ends_with('/')
            || raw.contains('\\')
            || raw.bytes().any(|byte| byte.is_ascii_control())
        {
            return Err(error(FitErrorId::InvalidPath));
        }
        let mut depth = 0;
        for component in raw.split('/') {
            depth += 1;
            if component.is_empty()
                || component == "."
                || component == ".."
                || component.len() > MAX_COMPONENT_BYTES
            {
                return Err(error(FitErrorId::InvalidPath));
            }
        }
        if depth > MAX_DEPTH {
            return Err(error(FitErrorId::InvalidPath));
        }
        Ok(Self(raw))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn folded(&self) -> String {
        self.0.to_ascii_lowercase()
    }

    pub(crate) fn components(&self) -> impl Iterator<Item = &str> {
        self.0.split('/')
    }
}

impl std::fmt::Debug for CanonicalPath {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_tuple("CanonicalPath")
            .field(&self.0)
            .finish()
    }
}
