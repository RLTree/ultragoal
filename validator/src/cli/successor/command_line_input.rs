use super::clap_grammar::ROOT_TOKEN;
use super::command_contract::{OutputMode, WorkspaceRoot};
use super::error::{ParseErrorId, ParseFailure};
use std::ffi::OsString;

pub(crate) fn take_workspace_root(
    args: &mut Vec<OsString>,
    failure_mode: OutputMode,
) -> Result<WorkspaceRoot, ParseFailure> {
    let mut root = WorkspaceRoot::workspace_default();
    let mut root_seen = false;
    let mut index = 0;
    while index < args.len() {
        let token = args[index]
            .to_str()
            .expect("prepared command-line arguments are UTF-8");
        if matches!(token, "--help" | "-h") {
            break;
        }
        if token.starts_with("--root=") {
            return fail(ParseErrorId::UnexpectedOptionValue, failure_mode);
        }
        if token != ROOT_TOKEN {
            index += 1;
            continue;
        }
        if root_seen {
            return fail(ParseErrorId::DuplicateOption, failure_mode);
        }
        let Some(value) = args.get(index + 1).and_then(|value| value.to_str()) else {
            return fail(ParseErrorId::MissingOptionValue, failure_mode);
        };
        if value.starts_with('-') {
            return fail(ParseErrorId::MissingOptionValue, failure_mode);
        }
        root = WorkspaceRoot::from_option_value(value)
            .ok_or_else(|| failure(ParseErrorId::InvalidPath, failure_mode))?;
        root_seen = true;
        args.drain(index..=index + 1);
    }
    Ok(root)
}

const fn failure(id: ParseErrorId, output_mode: OutputMode) -> ParseFailure {
    ParseFailure::new(id, output_mode)
}

fn fail<T>(id: ParseErrorId, output_mode: OutputMode) -> Result<T, ParseFailure> {
    Err(failure(id, output_mode))
}
