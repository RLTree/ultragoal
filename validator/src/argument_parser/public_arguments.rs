use super::*;
use std::ffi::OsString;

pub(crate) fn parse_public_os_args_from(
    raw: Vec<OsString>,
) -> Result<Args, cli::successor::ParseFailure> {
    let parsed: cli::successor::ParsedCommandLine = cli::successor::parse_command_line(raw)?;
    let (root, outcome): (cli::successor::WorkspaceRoot, cli::successor::ParseOutcome) =
        parsed.into_parts();
    Ok(Args {
        root: root.into_path_buf(),
        outcome,
    })
}
