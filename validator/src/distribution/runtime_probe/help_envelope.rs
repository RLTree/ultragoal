pub fn canonical_runtime_help_json() -> String {
    crate::cli::successor::render_help(
        crate::cli::successor::command_contract::HelpTarget::Root,
        crate::cli::successor::OutputMode::Json,
    )
}

fn validate_envelope(bytes: &[u8]) -> Result<(), DistributionError> {
    let actual: serde_json::Value = json::parse(bytes, 64 * 1024)?;
    let expected: serde_json::Value = serde_json::from_str(&canonical_runtime_help_json())
        .map_err(|_| error(DistributionErrorId::ProvenanceMismatch))?;
    if actual != expected {
        return Err(error(DistributionErrorId::ProvenanceMismatch));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stripped_help_catalog_cannot_satisfy_runtime_probe() {
        let stripped = br#"{"schema_version":"harness-ultragoal.cli-help.v1","grammar_version":"successor-v1-candidate","commands":[]}"#;
        assert_eq!(
            validate_envelope(stripped).unwrap_err().id(),
            DistributionErrorId::ProvenanceMismatch
        );
    }
}
