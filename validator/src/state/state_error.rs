use std::fmt;

#[derive(Debug, Eq, PartialEq)]
pub enum StateError {
    InvalidCatalog(String),
    ResourceLimit(String),
    Serialization(String),
    StaleContext(String),
}

impl fmt::Display for StateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCatalog(message) => write!(formatter, "invalid state catalog: {message}"),
            Self::ResourceLimit(message) => write!(formatter, "state resource limit: {message}"),
            Self::Serialization(message) => {
                write!(formatter, "state serialization failed: {message}")
            }
            Self::StaleContext(message) => write!(formatter, "live context is stale: {message}"),
        }
    }
}

impl std::error::Error for StateError {}
