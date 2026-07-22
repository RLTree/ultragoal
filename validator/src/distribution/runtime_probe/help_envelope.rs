#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProbeEnvelope {
    schema_version: String,
    grammar_version: String,
    commands: Vec<ProbeCommand>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProbeCommand {
    group: String,
    subcommand: Option<String>,
    effect: String,
    purpose: String,
    options: Vec<serde_json::Value>,
}

fn validate_envelope(row: &ProbeEnvelope) -> Result<(), DistributionError> {
    if row.schema_version != crate::cli::successor::HELP_SCHEMA
        || row.grammar_version != crate::cli::successor::SUCCESSOR_GRAMMAR_VERSION
        || !row.commands.iter().any(|command| {
            command.group == "inspect"
                && command.subcommand.as_deref() == Some("inception")
                && command.effect == "read"
                && !command.purpose.is_empty()
                && command.options.is_empty()
        })
    {
        return Err(error(DistributionErrorId::ProvenanceMismatch));
    }
    Ok(())
}
