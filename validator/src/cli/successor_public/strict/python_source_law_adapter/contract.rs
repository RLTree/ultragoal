use std::path::Path;

pub(in crate::cli::successor_public::strict) struct PythonSourceLawRequest;

impl PythonSourceLawRequest {
    pub(in crate::cli::successor_public::strict) fn bind(_root: &Path) -> Self {
        Self
    }
}

pub(in crate::cli::successor_public::strict) enum PythonSourceLawAdapterError {
    TrustedValidationUnavailable,
}

impl PythonSourceLawAdapterError {
    pub(in crate::cli::successor_public::strict) const fn id(&self) -> &'static str {
        match self {
            Self::TrustedValidationUnavailable => {
                "trusted_compiled_python_source_law_validation_unavailable"
            }
        }
    }
}
