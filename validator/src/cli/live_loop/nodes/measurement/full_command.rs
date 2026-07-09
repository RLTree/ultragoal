use super::super::super::surfaces::LoopValidationSurface;
use super::super::command_failure::CommandFailureSummary;
pub(super) use super::command_run::FullCommandRun;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

pub(super) fn run_full_command(root: &Path, surface: LoopValidationSurface) -> FullCommandRun {
    run_surface_command(root, surface.canonical_full_command)
}

pub(super) fn run_narrow_command(root: &Path, surface: LoopValidationSurface) -> FullCommandRun {
    run_surface_command(root, surface.narrow_rerun)
}

fn run_surface_command(root: &Path, command_text: &str) -> FullCommandRun {
    let argv = runtime_command_argv(command_text);
    run_surface_command_with_argv(root, &argv)
}

fn run_surface_command_with_argv(root: &Path, argv: &[String]) -> FullCommandRun {
    let started = Instant::now();
    let (program, args) = nonempty_runtime_argv(argv);
    let output = match Command::new(program).args(args).current_dir(root).output() {
        Ok(output) => output,
        Err(err) => {
            let runtime_command = argv.join(" ");
            return FullCommandRun {
                exit_code: 1,
                status_success: false,
                launch_error: true,
                duration_ms: elapsed_ms(started),
                stdout_digest: crate::digest::bytes(&[]),
                stderr_digest: crate::digest::bytes(
                    format!("{runtime_command} launch failed: {err}").as_bytes(),
                ),
                executed_test_count: None,
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
        executed_test_count: executed_test_count(&output.stdout),
        failure,
    }
}

fn executed_test_count(stdout: &[u8]) -> Option<u64> {
    let count = String::from_utf8_lossy(stdout)
        .lines()
        .filter_map(running_test_count)
        .sum::<u64>();
    (count > 0).then_some(count)
}

fn running_test_count(line: &str) -> Option<u64> {
    let rest = line.trim().strip_prefix("running ")?;
    let raw = rest
        .strip_suffix(" tests")
        .or_else(|| rest.strip_suffix(" test"))?;
    raw.parse().ok()
}

fn nonempty_runtime_argv(argv: &[String]) -> (&String, &[String]) {
    argv.split_first()
        .expect("runtime_command_argv always returns a command program")
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

pub(super) fn runtime_command_argv(command_text: &str) -> Vec<String> {
    runtime_command_argv_with_executable_context(
        command_text,
        std::env::var_os("CARGO_BIN_EXE_ultragoal"),
        std::env::current_exe().ok(),
    )
}

pub(super) fn runtime_command_argv_with_executable_context(
    command_text: &str,
    cargo_binary: Option<OsString>,
    current_exe: Option<PathBuf>,
) -> Vec<String> {
    if let Some(rest) = command_text.strip_prefix("target/debug/ultragoal") {
        if let Some(ultragoal_exe) = runtime_ultragoal_executable(cargo_binary, current_exe) {
            let mut argv = vec![ultragoal_exe.display().to_string()];
            argv.extend(split_simple_args(rest.trim_start()));
            return argv;
        }
    }
    if requires_developer_shell_resolution(command_text) {
        return vec![
            "bash".to_string(),
            "-lc".to_string(),
            runtime_command_text(command_text),
        ];
    }
    match split_typed_runtime_command(command_text) {
        Some(argv) => argv,
        None => vec![
            "bash".to_string(),
            "-lc".to_string(),
            runtime_command_text(command_text),
        ],
    }
}

pub(super) fn product_command_text(command_text: &str) -> String {
    product_command_argv(command_text).join(" ")
}

pub(super) fn product_command_argv(command_text: &str) -> Vec<String> {
    if let Some(rest) = command_text.strip_prefix("target/debug/ultragoal") {
        let mut argv = vec!["ultragoal".to_string()];
        argv.extend(split_simple_args(rest.trim_start()));
        return argv;
    }
    match split_typed_runtime_command(command_text) {
        Some(argv) => argv,
        None => vec![
            "bash".to_string(),
            "-lc".to_string(),
            command_text.to_string(),
        ],
    }
}

fn split_typed_runtime_command(command_text: &str) -> Option<Vec<String>> {
    if command_text.trim().is_empty() || requires_shell(command_text) {
        return None;
    }
    let argv = split_simple_args(command_text);
    if argv.is_empty() { None } else { Some(argv) }
}

fn split_simple_args(command_text: &str) -> Vec<String> {
    command_text
        .split_whitespace()
        .map(ToString::to_string)
        .collect()
}

fn requires_shell(command_text: &str) -> bool {
    command_text.chars().any(|ch| {
        matches!(
            ch,
            '|' | '&' | ';' | '<' | '>' | '(' | ')' | '$' | '`' | '"' | '\''
        )
    })
}

fn requires_developer_shell_resolution(command_text: &str) -> bool {
    matches!(command_text, "cargo" | "ultragoal")
        || command_text.starts_with("cargo ")
        || command_text.starts_with("ultragoal ")
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
