use super::super::super::surfaces::LoopValidationSurface;
use super::super::command_failure::CommandFailureSummary;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

pub(super) fn run_full_command(root: &Path, surface: LoopValidationSurface) -> FullCommandRun {
    run_surface_command_with_shell(root, surface.canonical_full_command, "bash")
}

#[cfg(test)]
pub(super) fn run_full_command_with_shell(
    root: &Path,
    surface: LoopValidationSurface,
    shell: &str,
) -> FullCommandRun {
    run_surface_command_with_shell(root, surface.canonical_full_command, shell)
}

pub(super) fn run_narrow_command(root: &Path, surface: LoopValidationSurface) -> FullCommandRun {
    run_surface_command_with_shell(root, surface.narrow_rerun, "bash")
}

fn run_surface_command_with_shell(root: &Path, command_text: &str, shell: &str) -> FullCommandRun {
    let started = Instant::now();
    let runtime_command = runtime_command_text(command_text);
    let output = match Command::new(shell)
        .arg("-lc")
        .arg(&runtime_command)
        .current_dir(root)
        .output()
    {
        Ok(output) => output,
        Err(err) => {
            return FullCommandRun {
                exit_code: 1,
                status_success: false,
                launch_error: true,
                duration_ms: elapsed_ms(started),
                stdout_digest: crate::digest::bytes(&[]),
                stderr_digest: crate::digest::bytes(
                    format!("{runtime_command} launch failed: {err}").as_bytes(),
                ),
                failure: CommandFailureSummary::default(),
            };
        }
    };
    let failure = CommandFailureSummary::from_stdout(&output.stdout);
    FullCommandRun {
        exit_code: output.status.code().unwrap_or(1),
        status_success: output.status.success(),
        launch_error: false,
        duration_ms: elapsed_ms(started),
        stdout_digest: crate::digest::bytes(&output.stdout),
        stderr_digest: crate::digest::bytes(&output.stderr),
        failure,
    }
}

pub(super) fn runtime_command_text(command_text: &str) -> String {
    runtime_command_text_with_executable_context(
        command_text,
        std::env::var_os("CARGO_BIN_EXE_ultragoal"),
        std::env::current_exe().ok(),
    )
}

pub(super) fn runtime_command_text_with_executable_context(
    command_text: &str,
    cargo_binary: Option<OsString>,
    current_exe: Option<PathBuf>,
) -> String {
    let Some(rest) = command_text.strip_prefix("target/debug/ultragoal") else {
        return command_text.to_string();
    };
    let Some(ultragoal_exe) = runtime_ultragoal_executable(cargo_binary, current_exe) else {
        return command_text.to_string();
    };
    format!(
        "{}{}",
        shell_quote(&ultragoal_exe.display().to_string()),
        rest
    )
}

pub(super) fn runtime_shell_argv(command_text: &str) -> Vec<String> {
    vec![
        "bash".to_string(),
        "-lc".to_string(),
        runtime_command_text(command_text),
    ]
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn runtime_ultragoal_executable(
    cargo_binary: Option<OsString>,
    current_exe: Option<PathBuf>,
) -> Option<PathBuf> {
    if let Some(path) = cargo_binary {
        return Some(path.into());
    }
    let current_exe = current_exe?;
    let file_name = current_exe.file_name().and_then(|name| name.to_str());
    if file_name == Some("ultragoal") {
        return Some(current_exe);
    }
    let adjacent_binary = current_exe.parent()?.parent()?.join("ultragoal");
    if adjacent_binary.is_file() {
        return Some(adjacent_binary);
    }
    Some(current_exe)
}

fn elapsed_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis())
        .unwrap_or(u64::MAX)
        .max(1)
}

#[derive(Clone, Debug)]
pub(super) struct FullCommandRun {
    pub(super) exit_code: i32,
    pub(super) status_success: bool,
    pub(super) launch_error: bool,
    pub(super) duration_ms: u64,
    pub(super) stdout_digest: String,
    pub(super) stderr_digest: String,
    pub(super) failure: CommandFailureSummary,
}
