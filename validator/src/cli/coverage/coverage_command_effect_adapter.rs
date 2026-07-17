#[cfg(test)]
use std::io;
use std::path::Path;
#[cfg(test)]
use std::process::Output;

pub(crate) struct CoverageCommandEffectResponse {
    pub(crate) code: i32,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
    pub(crate) cache_mode: &'static str,
}

impl CoverageCommandEffectResponse {
    pub(crate) fn validate_existing() -> Self {
        Self {
            code: 0,
            stdout: String::new(),
            stderr: String::new(),
            cache_mode: "coverage_validate_existing_no_cache",
        }
    }
}

pub(crate) fn execute_authoritative(root: &Path, receipt: &Path) -> CoverageCommandEffectResponse {
    match crate::package::inventory::package_digest(root) {
        Ok(candidate) => super::strict_observation::execute(root, receipt, &candidate),
        Err(error) => CoverageCommandEffectResponse {
            code: 2,
            stdout: String::new(),
            stderr: format!("coverage_package_digest_unavailable:{error}"),
            cache_mode: "coverage_strict_no_cache",
        },
    }
}

#[cfg(test)]
pub(crate) fn execution_from_output(result: io::Result<Output>) -> CoverageCommandEffectResponse {
    match result {
        Ok(output) => CoverageCommandEffectResponse {
            code: output.status.code().unwrap_or(2),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            cache_mode: "coverage_authoritative_no_cache",
        },
        Err(error) => CoverageCommandEffectResponse {
            code: 2,
            stdout: String::new(),
            stderr: format!("coverage_command_spawn_failed:{error}"),
            cache_mode: "coverage_authoritative_no_cache",
        },
    }
}

pub(crate) fn first_diagnostic(execution: &CoverageCommandEffectResponse) -> String {
    let lines = execution
        .stderr
        .lines()
        .chain(execution.stdout.lines())
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    lines
        .iter()
        .find(|line| is_actionable_diagnostic(line))
        .or_else(|| lines.iter().find(|line| !is_noise_diagnostic(line)))
        .or_else(|| lines.first())
        .copied()
        .unwrap_or("coverage command exited nonzero")
        .to_string()
}

pub(crate) fn is_noise_diagnostic(line: &str) -> bool {
    line.starts_with("info:") || line.starts_with("warning:")
}

fn is_actionable_diagnostic(line: &str) -> bool {
    line.contains("coverage_")
        || line.contains("error:")
        || line.contains("failed")
        || line.contains("panicked")
}
