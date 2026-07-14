use super::super::command_contract::{LegacyCommand, OutputMode};

pub(crate) const COMPATIBILITY_SCHEMA: &str = "harness-ultragoal.compatibility-guidance.v1";
pub(crate) const COMPATIBILITY_WARNING_ID: &str = "CLI_LEGACY_ROUTE_GUIDANCE";
pub(crate) const COMPATIBILITY_EXIT_CODE: i32 = 4;

pub(crate) fn render_compatibility_guidance(
    command: LegacyCommand,
    output_mode: OutputMode,
) -> String {
    match output_mode {
        OutputMode::Human => format!(
            "{COMPATIBILITY_WARNING_ID}: legacy command {} is non-authoritative; canonical target: {}; no legacy effect was executed; compatibility and retirement remain unproved",
            command.stable_name(),
            command.canonical_target()
        ),
        OutputMode::Json => format!(
            "{{\"schema_version\":\"{COMPATIBILITY_SCHEMA}\",\"warning_id\":\"{COMPATIBILITY_WARNING_ID}\",\"legacy_command\":\"{}\",\"canonical_target\":\"{}\",\"disposition\":\"guidance_only\",\"legacy_effect_executed\":false,\"exit_code\":{COMPATIBILITY_EXIT_CODE},\"claim_ceiling\":\"compatibility_and_retirement_unproved\"}}",
            command.stable_name(),
            command.canonical_target()
        ),
    }
}
