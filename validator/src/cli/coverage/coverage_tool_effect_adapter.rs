use super::contract_codec::ToolVersion;
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Command;

pub(crate) enum CoverageToolEffectRequest<'a> {
    RunRoutine {
        root: &'a Path,
        report: &'a Path,
        target_dir: &'a str,
    },
    RunStrict {
        root: &'a Path,
        report: &'a Path,
        missing_report: &'a Path,
        target_dir: &'a str,
    },
    ReadVersion {
        tool: CoverageTool,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CoverageTool {
    Cargo,
    CargoLlvmCov,
    Rustc,
}

pub(crate) enum CoverageToolEffectResponse {
    RoutineCompleted,
    StrictCompleted,
    Version(ToolVersion),
}

#[derive(Debug)]
pub(crate) struct CoverageToolEffectError {
    pub(crate) code: &'static str,
    pub(crate) program: &'static str,
    pub(crate) detail: String,
    pub(crate) root: Option<PathBuf>,
}

impl fmt::Display for CoverageToolEffectError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let root = self
            .root
            .as_ref()
            .map(|root| root.display().to_string())
            .unwrap_or_else(|| "<process>".to_string());
        write!(
            formatter,
            "{}:{}:{}:{}",
            self.code, self.program, root, self.detail
        )
    }
}

pub(crate) fn execute(
    request: CoverageToolEffectRequest<'_>,
) -> Result<CoverageToolEffectResponse, CoverageToolEffectError> {
    match request {
        CoverageToolEffectRequest::RunRoutine {
            root,
            report,
            target_dir,
        } => run_routine(root, report, target_dir),
        CoverageToolEffectRequest::RunStrict {
            root,
            report,
            missing_report,
            target_dir,
        } => run_strict(root, report, missing_report, target_dir),
        CoverageToolEffectRequest::ReadVersion { tool } => read_version(tool),
    }
}

fn run_strict(
    root: &Path,
    report: &Path,
    missing_report: &Path,
    target_dir: &str,
) -> Result<CoverageToolEffectResponse, CoverageToolEffectError> {
    run_cargo(
        root,
        target_dir,
        &["llvm-cov", "clean", "--workspace"],
        "coverage_strict_clean_failed",
    )?;
    let report_text = report.to_string_lossy();
    run_cargo(
        root,
        target_dir,
        &[
            "llvm-cov",
            "--workspace",
            "--all-features",
            "--json",
            "--output-path",
            &report_text,
            "--offline",
        ],
        "coverage_strict_command_failed",
    )?;
    let missing_text = missing_report.to_string_lossy();
    run_cargo(
        root,
        target_dir,
        &[
            "llvm-cov",
            "report",
            "--text",
            "--show-missing-lines",
            "--output-path",
            &missing_text,
            "--offline",
        ],
        "coverage_strict_missing_report_failed",
    )?;
    Ok(CoverageToolEffectResponse::StrictCompleted)
}

fn run_cargo(
    root: &Path,
    target_dir: &str,
    arguments: &[&str],
    code: &'static str,
) -> Result<(), CoverageToolEffectError> {
    let output = Command::new("cargo")
        .args(arguments)
        .current_dir(root)
        .env("CARGO_TARGET_DIR", target_dir)
        .output()
        .map_err(|error| tool_error("coverage_tool_spawn_failed", "cargo", error, Some(root)))?;
    if output.status.success() {
        return Ok(());
    }
    let detail = String::from_utf8_lossy(&output.stderr)
        .lines()
        .chain(String::from_utf8_lossy(&output.stdout).lines())
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("cargo llvm-cov failed")
        .to_string();
    Err(tool_error(code, "cargo", detail, Some(root)))
}

fn run_routine(
    root: &Path,
    report: &Path,
    target_dir: &str,
) -> Result<CoverageToolEffectResponse, CoverageToolEffectError> {
    let output = Command::new("cargo")
        .arg("llvm-cov")
        .arg("--workspace")
        .arg("--all-features")
        .arg("--json")
        .arg("--summary-only")
        .arg("--output-path")
        .arg(report)
        .arg("--offline")
        .arg("--no-clean")
        .arg("--")
        .arg("coverage")
        .current_dir(root)
        .env("CARGO_TARGET_DIR", target_dir)
        .output()
        .map_err(|error| tool_error("coverage_tool_spawn_failed", "cargo", error, Some(root)))?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr)
            .lines()
            .chain(String::from_utf8_lossy(&output.stdout).lines())
            .map(str::trim)
            .find(|line| !line.is_empty())
            .unwrap_or("cargo llvm-cov routine failed")
            .to_string();
        return Err(tool_error(
            "coverage_routine_command_failed",
            "cargo",
            detail,
            Some(root),
        ));
    }
    Ok(CoverageToolEffectResponse::RoutineCompleted)
}

fn read_version(tool: CoverageTool) -> Result<CoverageToolEffectResponse, CoverageToolEffectError> {
    let (program, arguments): (&str, &[&str]) = match tool {
        CoverageTool::Cargo => ("cargo", &["--version"]),
        CoverageTool::CargoLlvmCov => ("cargo", &["llvm-cov", "--version"]),
        CoverageTool::Rustc => ("rustc", &["--version"]),
    };
    let output = Command::new(program)
        .args(arguments)
        .output()
        .map_err(|error| tool_error("coverage_tool_version_spawn_failed", program, error, None))?;
    if !output.status.success() {
        return Err(tool_error(
            "coverage_tool_version_failed",
            program,
            format!("exit={:?}", output.status.code()),
            None,
        ));
    }
    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if version.is_empty() {
        return Err(tool_error(
            "coverage_tool_version_empty",
            program,
            "empty stdout",
            None,
        ));
    }
    Ok(CoverageToolEffectResponse::Version(ToolVersion::new(
        version,
    )))
}

fn tool_error(
    code: &'static str,
    program: &'static str,
    detail: impl fmt::Display,
    root: Option<&Path>,
) -> CoverageToolEffectError {
    CoverageToolEffectError {
        code,
        program,
        detail: detail.to_string(),
        root: root.map(Path::to_path_buf),
    }
}
