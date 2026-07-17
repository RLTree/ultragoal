#[cfg(test)]
use super::ImpactedRustTestsCommand;
use std::path::{Path, PathBuf};

#[cfg(test)]
pub(super) fn parse(raw: &[String]) -> Result<Option<ImpactedRustTestsCommand>, String> {
    let args = match raw {
        [a, b, c, rest @ ..] if a == "loop" && b == "rust-tests" && c == "impacted" => rest,
        _ => return Ok(None),
    };
    Ok(Some(ImpactedRustTestsCommand {
        receipt: opt_path(args, "--receipt").unwrap_or_else(|| {
            PathBuf::from("validation_artifacts/observability/impacted-rust-tests.json")
        }),
        jobs: opt_jobs(args, "--jobs")?,
        changed_paths: opt_paths(args, "--changed")?,
        run_tests: has_flag(args, "--run"),
    }))
}

pub(super) fn output_path(root: &Path, path: &Path, label: &str) -> Result<PathBuf, String> {
    match crate::output_path::claim_artifact_path(root, path, label) {
        Ok(path) => Ok(path),
        Err(_err) if path.starts_with("target/") => {
            let debug_path = root.join(path);
            crate::output_path::prepare_parent(&debug_path)?;
            Ok(debug_path)
        }
        Err(err) => Err(err),
    }
}

#[cfg(test)]
fn opt_path(args: &[String], key: &str) -> Option<PathBuf> {
    args.windows(2)
        .find(|window| window[0] == key)
        .map(|window| PathBuf::from(&window[1]))
}

#[cfg(test)]
fn opt_paths(args: &[String], key: &str) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            value if value == key => {
                let Some(path) = args.get(index + 1) else {
                    return Err(format!("missing value for {key}"));
                };
                out.push(PathBuf::from(path));
                index += 2;
            }
            "--run" => index += 1,
            "--jobs" | "--receipt" => index += 2,
            other => return Err(format!("unknown impacted Rust test argument: {other}")),
        }
    }
    Ok(out)
}

#[cfg(test)]
fn opt_jobs(args: &[String], key: &str) -> Result<Option<usize>, String> {
    match args
        .windows(2)
        .find(|window| window[0] == key)
        .map(|w| w[1].as_str())
    {
        None | Some("auto") => Ok(None),
        Some(raw) => raw
            .parse()
            .map(Some)
            .map_err(|_| format!("invalid --jobs value: {raw}")),
    }
}

#[cfg(test)]
fn has_flag(args: &[String], key: &str) -> bool {
    args.iter().any(|arg| arg == key)
}
