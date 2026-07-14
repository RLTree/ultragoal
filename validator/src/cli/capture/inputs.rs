use std::ffi::OsString;
use std::path::PathBuf;

pub struct PublicArg {
    pub(super) value: OsString,
}

impl PublicArg {
    pub fn new(value: impl Into<OsString>) -> Self {
        Self {
            value: value.into(),
        }
    }
}

pub struct SecretArg {
    pub(super) value: OsString,
    pub(super) source: String,
}

impl SecretArg {
    pub fn new(value: impl Into<OsString>, bound_source: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            source: bound_source.into(),
        }
    }
}

pub struct PublicEnv {
    pub(super) name: OsString,
    pub(super) value: OsString,
}

impl PublicEnv {
    pub fn new(name: impl Into<OsString>, value: impl Into<OsString>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }
}

pub struct SecretEnv {
    pub(super) name: OsString,
    pub(super) value: OsString,
    pub(super) source: String,
}

impl SecretEnv {
    pub fn new(
        name: impl Into<OsString>,
        value: impl Into<OsString>,
        bound_source: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            source: bound_source.into(),
        }
    }
}

pub struct PublicArtifact {
    pub(super) path: PathBuf,
    pub(super) expected_sha256: Option<String>,
}

impl PublicArtifact {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            expected_sha256: None,
        }
    }

    pub fn with_digest(path: impl Into<PathBuf>, sha256: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            expected_sha256: Some(sha256.into()),
        }
    }
}

pub struct SecretArtifact {
    pub(super) path: PathBuf,
    pub(super) source: String,
}

impl SecretArtifact {
    pub fn new(path: impl Into<PathBuf>, bound_source: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            source: bound_source.into(),
        }
    }
}

pub(crate) enum ArgumentInput {
    Public(PublicArg),
    Secret(SecretArg),
}

pub(crate) enum EnvironmentInput {
    Public(PublicEnv),
    Secret(SecretEnv),
}

pub(crate) enum ArtifactExpectation {
    Public(PublicArtifact),
    Secret(SecretArtifact),
}
