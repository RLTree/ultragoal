use super::contract::{PythonSourceLawAdapterError, PythonSourceLawRequest};

pub(in crate::cli::successor_public::strict) fn run(
    _request: PythonSourceLawRequest,
) -> Result<Vec<String>, PythonSourceLawAdapterError> {
    Err(super::diagnostic::trusted_validation_unavailable())
}
