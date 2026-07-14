use crate::context::ToolCapability;
use std::path::{Path, PathBuf};

pub(in crate::cli::successor_public::strict) struct PythonSourceLawRequest {
    pub(super) root: PathBuf,
    pub(super) executable: PathBuf,
    pub(super) executable_sha256: String,
}

impl PythonSourceLawRequest {
    pub(in crate::cli::successor_public::strict) fn bind(
        root: &Path,
        capability: Option<&ToolCapability>,
    ) -> Result<Self, PythonSourceLawAdapterError> {
        let capability = capability
            .filter(|value| value.available)
            .ok_or(PythonSourceLawAdapterError::CapabilityMissing)?;
        let executable = PathBuf::from(
            capability
                .executable
                .as_deref()
                .ok_or(PythonSourceLawAdapterError::CapabilityMissing)?,
        );
        let executable_sha256 = capability
            .executable_sha256
            .clone()
            .ok_or(PythonSourceLawAdapterError::CapabilityMissing)?;
        Ok(Self {
            root: root.to_path_buf(),
            executable,
            executable_sha256,
        })
    }
}

pub(in crate::cli::successor_public::strict) struct PythonSourceLawResponse {
    findings: Vec<String>,
}

impl PythonSourceLawResponse {
    pub(in crate::cli::successor_public::strict) fn new(findings: Vec<String>) -> Self {
        Self { findings }
    }

    pub(in crate::cli::successor_public::strict) fn into_findings(self) -> Vec<String> {
        self.findings
    }
}

pub(in crate::cli::successor_public::strict) enum PythonSourceLawAdapterError {
    CapabilityMissing,
    IdentityMismatch,
    ScriptUnavailable,
    SpawnFailed,
    TimedOut,
    OutputTooLarge,
    OutputInvalid,
    UnexpectedExit,
}

impl PythonSourceLawAdapterError {
    pub(in crate::cli::successor_public::strict) const fn id(&self) -> &'static str {
        match self {
            Self::CapabilityMissing => "python_capability_missing",
            Self::IdentityMismatch => "python_identity_mismatch",
            Self::ScriptUnavailable => "python_source_law_script_unavailable",
            Self::SpawnFailed => "python_source_law_spawn_failed",
            Self::TimedOut => "python_source_law_timeout",
            Self::OutputTooLarge => "python_source_law_output_too_large",
            Self::OutputInvalid => "python_source_law_output_invalid",
            Self::UnexpectedExit => "python_source_law_unexpected_exit",
        }
    }
}
