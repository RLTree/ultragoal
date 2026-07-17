use std::path::{Path, PathBuf};

const DEFAULT_REPORT: &str = "validation_artifacts/ultragoal-audit/red-fixture-report.json";

#[derive(Debug)]
pub(crate) struct RedReportCommand {
    pub(crate) report: PathBuf,
}

#[cfg(test)]
pub(crate) fn parse(raw: &[String]) -> Result<Option<RedReportCommand>, String> {
    let args = match raw {
        [first, second, third, rest @ ..]
            if first == "red" && second == "fixture" && third == "report" =>
        {
            rest
        }
        [first, rest @ ..] if first == "red-fixture-report" => rest,
        _ => return Ok(None),
    };
    let report = opt_report(args)?;
    Ok(Some(RedReportCommand {
        report: report.unwrap_or_else(|| PathBuf::from(DEFAULT_REPORT)),
    }))
}

pub(crate) fn run(root: &Path, command: &RedReportCommand) -> Result<i32, String> {
    crate::cli::audit::run_red_fixture_report(root, &command.report)
}

#[cfg(test)]
fn opt_report(args: &[String]) -> Result<Option<PathBuf>, String> {
    let mut report = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--report" | "--red-report" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| format!("missing value for {}", args[index]))?;
                report = Some(PathBuf::from(value));
                index += 2;
            }
            other => return Err(format!("unknown red fixture report argument: {other}")),
        }
    }
    Ok(report)
}

#[cfg(test)]
mod command_surface_tests;
