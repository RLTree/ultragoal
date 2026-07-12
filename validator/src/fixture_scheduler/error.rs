use std::fmt;
use std::io;

#[derive(Debug)]
pub enum FixtureScheduleError {
    Cleanup { lease_id: String, source: io::Error },
    Collision(String),
    ExpectationMismatch { expected: String, observed: String },
    Integrity(String),
    InvalidMetadata(String),
    Io(io::Error),
    UnknownLease(String),
}

impl FixtureScheduleError {
    pub(crate) fn cleanup(lease_id: &str, source: io::Error) -> Self {
        Self::Cleanup {
            lease_id: lease_id.to_owned(),
            source,
        }
    }
}

impl fmt::Display for FixtureScheduleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cleanup { lease_id, source } => {
                write!(formatter, "cleanup failed for {lease_id}: {source}")
            }
            Self::Collision(value) => write!(formatter, "fixture isolation collision: {value}"),
            Self::ExpectationMismatch { expected, observed } => {
                write!(formatter, "expected {expected}, observed {observed}")
            }
            Self::Integrity(value) => {
                write!(formatter, "fixture expectation integrity failed: {value}")
            }
            Self::InvalidMetadata(value) => write!(formatter, "invalid fixture metadata: {value}"),
            Self::Io(source) => write!(formatter, "fixture isolation I/O failed: {source}"),
            Self::UnknownLease(value) => write!(formatter, "unknown fixture lease: {value}"),
        }
    }
}

impl std::error::Error for FixtureScheduleError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Cleanup { source, .. } | Self::Io(source) => Some(source),
            _ => None,
        }
    }
}

impl From<io::Error> for FixtureScheduleError {
    fn from(source: io::Error) -> Self {
        Self::Io(source)
    }
}
