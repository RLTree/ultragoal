use super::contract::PythonSourceLawAdapterError;

pub(super) const fn trusted_validation_unavailable() -> PythonSourceLawAdapterError {
    PythonSourceLawAdapterError::TrustedValidationUnavailable
}

#[cfg(test)]
mod tests {
    use super::trusted_validation_unavailable;

    #[test]
    fn unavailable_validator_has_a_stable_actionable_id() {
        assert_eq!(
            trusted_validation_unavailable().id(),
            "trusted_compiled_python_source_law_validation_unavailable"
        );
    }
}
