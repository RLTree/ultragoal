use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::Instant;

mod status_paths;
mod stdout;
use status_paths::{
    changed_paths_from_status, is_package_owned_rustfmt_input, is_rust_source, is_rustfmt_config,
};
use stdout::FormatResult;

const RUSTFMT_EDITION: &str = "2024";

pub(crate) fn run(root: &Path) -> Result<i32, String> {
    let captured = capture(root);
    print!("{}", String::from_utf8_lossy(&captured.stdout));
    Ok(captured.exit_code)
}

pub(crate) fn capture(root: &Path) -> CapturedFormatCheck {
    let started = Instant::now();
    let plan = format_plan(root);
    let result = match plan {
        Ok(FormatPlan::NoChangedRust) => FormatResult::pass(
            "changed-rust",
            0,
            elapsed_ms(started),
            "no changed Rust source files require routine formatting",
        ),
        Ok(FormatPlan::ChangedRust(files)) => run_rustfmt(root, &files, started),
        Ok(FormatPlan::WorkspaceConfigChanged) => run_cargo_fmt(root, started),
        Err(err) => FormatResult::blocked(
            "changed-input-discovery",
            elapsed_ms(started),
            "rust_format_status_unavailable",
            "loop.format_check.changed_inputs",
            err,
            "run the routine formatter from the package git root, or repair git status access before rerunning `target/debug/ultragoal --root . loop format check --changed-rust`",
        ),
    };
    let stdout = format!("{}\n", result.stdout_line()).into_bytes();
    CapturedFormatCheck {
        exit_code: result.exit_code,
        stdout,
    }
}

fn format_plan(root: &Path) -> Result<FormatPlan, String> {
    let status = git_status(root)?;
    let paths = changed_paths_from_status(&status);
    if paths.iter().any(|path| is_rustfmt_config(path)) {
        return Ok(FormatPlan::WorkspaceConfigChanged);
    }
    let files = paths
        .into_iter()
        .filter(|path| is_rust_source(path))
        .filter(|path| is_package_owned_rustfmt_input(root, path))
        .collect::<Vec<_>>();
    if files.is_empty() {
        Ok(FormatPlan::NoChangedRust)
    } else {
        Ok(FormatPlan::ChangedRust(files))
    }
}

fn git_status(root: &Path) -> Result<String, String> {
    ensure_git_root_matches_requested_root(root)?;
    git_status_text(git_status_output(root))
}

fn git_status_output(root: &Path) -> std::io::Result<Output> {
    Command::new("git")
        .args(["status", "--porcelain=v1", "-z", "--untracked-files=all"])
        .current_dir(root)
        .output()
}

fn git_status_text(output: std::io::Result<Output>) -> Result<String, String> {
    let output = output
        .map_err(|err| format!("git status could not launch for routine formatting: {err}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(git_failure_message(
            &output.stderr,
            "git status failed while discovering changed Rust files for routine formatting",
            "git status failed while discovering changed Rust files",
        ))
    }
}

fn ensure_git_root_matches_requested_root(root: &Path) -> Result<(), String> {
    let requested = root
        .canonicalize()
        .map_err(|err| format!("routine formatting root is not accessible: {err}"))?;
    let git_root = git_root_text(git_root_output(root))?;
    if requested == git_root {
        Ok(())
    } else {
        Err(format!(
            "routine formatting root must be the package git root; requested={} git_root={}",
            requested.display(),
            git_root.display()
        ))
    }
}

fn git_root_text(output: std::io::Result<Output>) -> Result<PathBuf, String> {
    let output = output
        .map_err(|err| format!("git root could not be resolved for routine formatting: {err}"))?;
    if !output.status.success() {
        return Err(git_failure_message(
            &output.stderr,
            "routine formatting root is not inside a git worktree",
            "routine formatting root is not an exact package git root",
        ));
    }
    git_root_from_stdout(output.stdout)
}

fn git_root_output(root: &Path) -> std::io::Result<Output> {
    Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(root)
        .output()
}

fn git_failure_message(stderr: &[u8], empty_message: &str, prefixed_message: &str) -> String {
    let stderr = String::from_utf8_lossy(stderr);
    let reason = stderr.trim();
    if reason.is_empty() {
        empty_message.to_string()
    } else {
        format!("{prefixed_message}: {reason}")
    }
}

fn git_root_from_stdout(stdout: Vec<u8>) -> Result<PathBuf, String> {
    String::from_utf8(stdout)
        .ok()
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
        .and_then(|text| Path::new(&text).canonicalize().ok())
        .ok_or_else(|| "git root output was empty or unreadable for routine formatting".to_string())
}

fn run_rustfmt(root: &Path, files: &[PathBuf], started: Instant) -> FormatResult {
    let mut command = Command::new("rustfmt");
    command
        .arg("--edition")
        .arg(RUSTFMT_EDITION)
        .arg("--check")
        .args(files)
        .current_dir(root);
    command_result(
        command.output(),
        "changed-rust",
        files.len(),
        elapsed_ms(started),
        "rust_format_changed_file_failed",
        "loop.format_check.changed_rust",
        "run rustfmt on the changed Rust source files, then rerun `target/debug/ultragoal --root . loop format check --changed-rust`",
    )
}

fn run_cargo_fmt(root: &Path, started: Instant) -> FormatResult {
    let mut command = Command::new("cargo");
    command.args(["fmt", "--all", "--check"]).current_dir(root);
    command_result(
        command.output(),
        "workspace-config",
        1,
        elapsed_ms(started),
        "rust_format_workspace_config_failed",
        "loop.format_check.workspace_config",
        "run `cargo fmt --all`, then rerun `target/debug/ultragoal --root . loop format check --changed-rust`",
    )
}

fn command_result(
    output: std::io::Result<std::process::Output>,
    mode: &'static str,
    work_unit_count: usize,
    duration_ms: u64,
    failure_class: &'static str,
    where_failed: &'static str,
    next_repair: &'static str,
) -> FormatResult {
    match output {
        Ok(output) if output.status.success() => FormatResult::pass(
            mode,
            work_unit_count,
            duration_ms,
            "rust formatting check passed for the routine affected set",
        ),
        Ok(output) => FormatResult::fail(
            mode,
            work_unit_count,
            duration_ms,
            output.status.code().unwrap_or(1),
            failure_class,
            where_failed,
            next_repair,
            &output.stdout,
            &output.stderr,
        ),
        Err(err) => FormatResult::fail(
            mode,
            work_unit_count,
            duration_ms,
            1,
            "rust_format_command_launch_failed",
            where_failed,
            "make rustfmt/cargo fmt available for the active Rust toolchain, then rerun `target/debug/ultragoal --root . loop format check --changed-rust`",
            &[],
            format!("{err}").as_bytes(),
        ),
    }
}

enum FormatPlan {
    NoChangedRust,
    ChangedRust(Vec<PathBuf>),
    WorkspaceConfigChanged,
}

pub(crate) struct CapturedFormatCheck {
    pub(crate) exit_code: i32,
    pub(crate) stdout: Vec<u8>,
}

fn elapsed_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis())
        .unwrap_or(u64::MAX)
        .max(1)
}

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
