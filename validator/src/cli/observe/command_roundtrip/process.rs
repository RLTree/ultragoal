use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug)]
pub(super) struct CommandOutput {
    pub(super) status_success: bool,
    pub(super) exit_code: i32,
    pub(super) stdout: String,
    pub(super) stderr: String,
}

pub(super) fn run_production_command(root: &Path, args: &[&str]) -> Result<CommandOutput, String> {
    run_production_command_with_exe(current_exe(), root, args)
}

fn run_production_command_with_exe(
    exe: Result<PathBuf, String>,
    root: &Path,
    args: &[&str],
) -> Result<CommandOutput, String> {
    run_with_exe(exe?, root, args)
}

fn current_exe() -> Result<PathBuf, String> {
    current_exe_with(
        std::env::var("ULTRAGOAL_FIT_EXE").ok(),
        std::env::current_exe,
    )
}

fn current_exe_with<F>(override_exe: Option<String>, current_exe: F) -> Result<PathBuf, String>
where
    F: FnOnce() -> std::io::Result<PathBuf>,
{
    if let Some(path) = override_exe {
        return Ok(PathBuf::from(path));
    }
    current_exe().map_err(|err| format!("current exe unavailable: {err}"))
}

fn run_with_exe(exe: PathBuf, root: &Path, args: &[&str]) -> Result<CommandOutput, String> {
    let output = Command::new(exe)
        .arg("--root")
        .arg(root)
        .args(args)
        .output()
        .map_err(|err| format!("production command launch failed: {err}"))?;
    Ok(CommandOutput {
        status_success: output.status.success(),
        exit_code: output.status.code().unwrap_or(1),
        stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::{
        current_exe_with, run_production_command, run_production_command_with_exe, run_with_exe,
    };
    use std::path::Path;

    #[test]
    fn process_adapter_captures_stdout_and_status() {
        let output = run_with_exe(
            std::path::PathBuf::from("/bin/echo"),
            Path::new("."),
            &["hello"],
        )
        .expect("echo runs");
        assert!(output.status_success);
        assert!(output.stdout.contains("--root"));
        assert!(output.stdout.contains("hello"));
        assert_eq!(output.stderr, "");
    }

    #[test]
    fn production_command_launcher_returns_captured_nonzero_status() {
        let output = run_production_command(
            Path::new("."),
            &["--definitely-not-a-real-ultragoal-test-argument"],
        )
        .expect("current test executable launches");
        assert!(!output.status_success);
        assert_ne!(output.exit_code, 0);
    }

    #[test]
    fn current_exe_resolution_reports_os_error_without_override() {
        let err = current_exe_with(None, || Err(std::io::Error::other("missing exe")))
            .expect_err("current exe error");
        assert!(err.contains("current exe unavailable"), "{err}");
    }

    #[test]
    fn current_exe_resolution_prefers_explicit_override() {
        let path = current_exe_with(Some("/bin/echo".to_string()), || {
            Err(std::io::Error::other("ignored"))
        })
        .expect("override wins");
        assert_eq!(path, std::path::PathBuf::from("/bin/echo"));
    }

    #[test]
    fn production_command_launcher_propagates_exe_resolution_failure() {
        let err = run_production_command_with_exe(
            Err("current exe unavailable: missing".to_string()),
            Path::new("."),
            &["package", "digest"],
        )
        .expect_err("exe resolution failure");
        assert!(err.contains("current exe unavailable"), "{err}");
    }

    #[test]
    fn process_adapter_reports_launch_failure() {
        let err = run_with_exe(
            std::path::PathBuf::from("/definitely/missing/ultragoal"),
            Path::new("."),
            &["package", "digest"],
        )
        .expect_err("launch fails");
        assert!(err.contains("production command launch failed"), "{err}");
    }
}
