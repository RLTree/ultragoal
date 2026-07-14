use super::super::command_contract::LegacyCommand;
use std::ffi::OsString;

#[path = "route_catalog.rs"]
mod route_catalog;

pub(crate) fn classify_legacy_command(args: &[OsString]) -> Option<LegacyCommand> {
    let tokens = args
        .iter()
        .filter_map(|argument| argument.to_str())
        .filter(|token| *token != "--json")
        .collect::<Vec<_>>();
    let command = route_catalog::classify(&tokens)?;
    if tokens
        .iter()
        .any(|token| matches!(*token, "help" | "--help" | "-h"))
    {
        Some(LegacyCommand::Help)
    } else {
        Some(command)
    }
}
