use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::observability::EventStore;
use crate::routine_work::{RoutineBinding, RoutineError, RoutineErrorId};

const COMMAND_TIMEOUT: Duration = Duration::from_secs(5);
const STORE_PARENT: &str = "validation_artifacts/observability/spool";

pub(crate) fn runtime_store_ignored(
    binding: &RoutineBinding,
    source_id: &str,
) -> Result<bool, RoutineError> {
    let git = binding
        .tool("git")
        .filter(|tool| tool.available())
        .and_then(|tool| tool.executable())
        .ok_or_else(|| capability_error("bound-git-unavailable"))?;
    if !git.is_absolute() {
        return Err(capability_error("bound-git-path-not-absolute"));
    }
    let relative = runtime_store_relative_path(binding, source_id)?;
    let status = run_git(
        git,
        binding,
        &["--no-optional-locks", "check-ignore", "--quiet", "--"],
        &relative,
        "git-check-ignore",
    )?;
    match status {
        Some(0) => runtime_store_untracked(git, binding),
        Some(1) => Ok(false),
        _ => Err(capture_cause("git-check-ignore-nonzero")),
    }
}

fn runtime_store_relative_path(
    binding: &RoutineBinding,
    source_id: &str,
) -> Result<PathBuf, RoutineError> {
    let leaf =
        EventStore::binding_leaf_name(binding.context_id(), binding.candidate_id(), source_id)
            .map_err(|_| capability_error("routine-runtime-observability-binding-invalid"))?;
    Ok(Path::new(STORE_PARENT).join(leaf))
}

fn runtime_store_untracked(git: &Path, binding: &RoutineBinding) -> Result<bool, RoutineError> {
    let status = run_git(
        git,
        binding,
        &["--no-optional-locks", "ls-files", "--error-unmatch", "--"],
        Path::new(STORE_PARENT),
        "git-ls-files",
    )?;
    match status {
        Some(1) => Ok(true),
        Some(0) => Ok(false),
        _ => Err(capture_cause("git-ls-files-nonzero")),
    }
}

fn run_git(
    git: &Path,
    binding: &RoutineBinding,
    args: &[&str],
    relative: &Path,
    operation: &'static str,
) -> Result<Option<i32>, RoutineError> {
    let mut child = Command::new(git)
        .args(args)
        .arg(relative)
        .current_dir(binding.worktree_root())
        .env_clear()
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| capture_error(operation, "spawn"))?;
    let deadline = Instant::now() + COMMAND_TIMEOUT;
    loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|_| capture_error(operation, "wait"))?
        {
            return Ok(status.code());
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(limit_error(operation));
        }
        thread::sleep(Duration::from_millis(2));
    }
}

fn capability_error(cause: &'static str) -> RoutineError {
    RoutineError::new(RoutineErrorId::CapabilityUnavailable, cause, None)
}

fn capture_error(operation: &'static str, phase: &'static str) -> RoutineError {
    let cause = match (operation, phase) {
        ("git-check-ignore", "spawn") => "git-check-ignore-spawn-failed",
        ("git-check-ignore", "wait") => "git-check-ignore-wait-failed",
        ("git-ls-files", "spawn") => "git-ls-files-spawn-failed",
        _ => "git-ls-files-wait-failed",
    };
    RoutineError::new(RoutineErrorId::CaptureFailed, cause, None)
}

fn capture_cause(cause: &'static str) -> RoutineError {
    RoutineError::new(RoutineErrorId::CaptureFailed, cause, None)
}

fn limit_error(operation: &'static str) -> RoutineError {
    let cause = match operation {
        "git-check-ignore" => "git-check-ignore-time-limit-exceeded",
        _ => "git-ls-files-time-limit-exceeded",
    };
    RoutineError::new(RoutineErrorId::CaptureLimit, cause, None)
}
