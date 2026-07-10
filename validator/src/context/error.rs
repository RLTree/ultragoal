use std::fmt;
use std::path::PathBuf;

#[derive(Debug)]
pub enum ContextError {
    InvalidRequest(String),
    Io {
        path: PathBuf,
        message: String,
    },
    Probe {
        program: String,
        message: String,
    },
    UnsupportedCapability(String),
    NotGitRepository(PathBuf),
    RootMismatch {
        dimension: &'static str,
        expected: PathBuf,
        actual: PathBuf,
    },
    ConcurrentMutation(String),
    EffectDenied(String),
    PathDenied(String),
    Serialization(String),
}

impl fmt::Display for ContextError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRequest(message) => {
                write!(formatter, "invalid context request: {message}")
            }
            Self::Io { path, message } => write!(formatter, "{}: {message}", path.display()),
            Self::Probe { program, message } => {
                write!(formatter, "{program} probe failed: {message}")
            }
            Self::UnsupportedCapability(message) => {
                write!(formatter, "unsupported capability: {message}")
            }
            Self::NotGitRepository(path) => {
                write!(formatter, "{} is not inside a Git worktree", path.display())
            }
            Self::RootMismatch {
                dimension,
                expected,
                actual,
            } => write!(
                formatter,
                "{dimension} root mismatch: expected {}, resolved {}",
                expected.display(),
                actual.display()
            ),
            Self::ConcurrentMutation(dimension) => {
                write!(
                    formatter,
                    "live context changed during construction: {dimension}"
                )
            }
            Self::EffectDenied(message) => write!(formatter, "effect denied: {message}"),
            Self::PathDenied(message) => write!(formatter, "path denied: {message}"),
            Self::Serialization(message) => {
                write!(formatter, "context serialization failed: {message}")
            }
        }
    }
}

impl std::error::Error for ContextError {}

pub(crate) fn io_error(path: impl Into<PathBuf>, error: impl fmt::Display) -> ContextError {
    ContextError::Io {
        path: path.into(),
        message: error.to_string(),
    }
}
