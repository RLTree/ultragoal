impl ActivationFailure {
    pub const fn code(self) -> &'static str {
        match self {
            Self::UnknownRow => "unknown-row",
            Self::MissingRow => "missing-row",
            Self::DuplicateRow => "duplicate-row",
            Self::ConflictingRow => "conflicting-row",
            Self::StaleInput => "stale-input",
            Self::UnsafeInput => "unsafe-input",
        }
    }
}

#[derive(Debug)]
pub enum InventoryError {
    Context(String),
    Io { path: PathBuf, message: String },
    Json { path: PathBuf, message: String },
    PathEscape(PathBuf),
    InvalidRegistry(String),
    Activation(ActivationFailure),
    Serialization(String),
}

impl fmt::Display for InventoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Context(message) => write!(formatter, "live context is stale: {message}"),
            Self::Io { path, message } => write!(formatter, "{}: {message}", path.display()),
            Self::Json { path, message } => write!(formatter, "{}: {message}", path.display()),
            Self::PathEscape(path) => {
                write!(formatter, "path escapes worktree: {}", path.display())
            }
            Self::InvalidRegistry(message) => write!(formatter, "invalid registry: {message}"),
            Self::Activation(failure) => {
                write!(formatter, "inventory activation failed: {}", failure.code())
            }
            Self::Serialization(message) => write!(formatter, "serialization failed: {message}"),
        }
    }
}

impl std::error::Error for InventoryError {}
