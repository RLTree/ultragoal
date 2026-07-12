use serde::Serialize;
use std::fmt;

use super::{RoutineError, RoutineErrorId};

#[derive(Clone, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct RepoPath(String);

impl RepoPath {
    pub fn parse(value: impl Into<String>) -> Result<Self, RoutineError> {
        let value = value.into();
        let depth = value.split('/').count();
        let invalid = value.is_empty()
            || value.len() > 4096
            || depth > 64
            || value.starts_with('/')
            || value.ends_with('/')
            || value.contains("//")
            || value.contains('\\')
            || value.bytes().any(|byte| byte.is_ascii_control())
            || value
                .split('/')
                .any(|component| component.is_empty() || matches!(component, "." | ".."));
        if invalid {
            return Err(RoutineError::new(
                RoutineErrorId::InvalidPath,
                "repository-relative-path-required",
                Some(value.as_bytes()),
            ));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn matches_prefix(&self, prefix: &Self) -> bool {
        self == prefix
            || self
                .0
                .strip_prefix(&prefix.0)
                .is_some_and(|suffix| suffix.starts_with('/'))
    }

    pub(crate) fn case_key(&self) -> String {
        self.0.to_ascii_lowercase()
    }
}

impl fmt::Debug for RepoPath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("RepoPath(<withheld>)")
    }
}
