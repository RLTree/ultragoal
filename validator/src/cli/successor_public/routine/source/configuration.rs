use super::*;

#[path = "mod.rs"]
mod source;

pub(crate) const SOURCE_CONFIG_KEY: &str = "contract_id";

pub(crate) fn execute(
    root: &Path,
    invocation: &ParsedInvocation,
    home: Option<&Path>,
) -> RuntimeOutcome {
    if let Some(outcome) = behavior_child::execute_if_requested(invocation) {
        return outcome;
    }
    source::execute_inner(root, invocation, home).unwrap_or_else(outcome::failure)
}
