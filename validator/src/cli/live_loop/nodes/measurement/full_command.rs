use super::super::super::surfaces::LoopValidationSurface;
use std::path::Path;
use std::process::Command;
use std::time::Instant;

pub(super) fn run_full_command(root: &Path, surface: LoopValidationSurface) -> FullCommandRun {
    run_full_command_with_shell(root, surface, "bash")
}

pub(super) fn run_full_command_with_shell(
    root: &Path,
    surface: LoopValidationSurface,
    shell: &str,
) -> FullCommandRun {
    let started = Instant::now();
    let output = match Command::new(shell)
        .arg("-lc")
        .arg(surface.canonical_full_command)
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
                    format!("{} launch failed: {err}", surface.canonical_full_command).as_bytes(),
                ),
            };
        }
    };
    FullCommandRun {
        exit_code: output.status.code().unwrap_or(1),
        status_success: output.status.success(),
        launch_error: false,
        duration_ms: elapsed_ms(started),
        stdout_digest: crate::digest::bytes(&output.stdout),
        stderr_digest: crate::digest::bytes(&output.stderr),
    }
}

fn elapsed_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis())
        .unwrap_or(u64::MAX)
        .max(1)
}

#[derive(Debug)]
pub(super) struct FullCommandRun {
    pub(super) exit_code: i32,
    pub(super) status_success: bool,
    pub(super) launch_error: bool,
    pub(super) duration_ms: u64,
    pub(super) stdout_digest: String,
    pub(super) stderr_digest: String,
}
