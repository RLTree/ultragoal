use std::io::Read;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::path::Path;
use std::sync::mpsc::{self, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

use crate::routine_work::{RoutineBinding, RoutineError, RoutineErrorId};

const STATUS_LIMIT: usize = 16 * 1024 * 1024;
const STATUS_TIMEOUT: Duration = Duration::from_secs(5);

pub(crate) fn status_bytes(binding: &RoutineBinding) -> Result<Vec<u8>, RoutineError> {
    let git = binding
        .tool("git")
        .filter(|tool| tool.available())
        .and_then(|tool| tool.executable())
        .ok_or_else(|| capability_error("bound-git-unavailable"))?;
    if !git.is_absolute() {
        return Err(capability_error("bound-git-path-not-absolute"));
    }
    let mut child = Command::new(git)
        .args([
            "--no-optional-locks",
            "status",
            "--porcelain=v2",
            "-z",
            "--untracked-files=all",
            "--ignore-submodules=none",
        ])
        .current_dir(binding.worktree_root())
        .env_clear()
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| capture_error("git-status-spawn-failed"))?;
    collect_bounded(&mut child)
}

pub(crate) fn runtime_store_ignored(binding: &RoutineBinding) -> Result<bool, RoutineError> {
    let git = binding
        .tool("git")
        .filter(|tool| tool.available())
        .and_then(|tool| tool.executable())
        .ok_or_else(|| capability_error("bound-git-unavailable"))?;
    if !git.is_absolute() {
        return Err(capability_error("bound-git-path-not-absolute"));
    }
    let mut child = Command::new(git)
        .args([
            "--no-optional-locks",
            "check-ignore",
            "--quiet",
            "validation_artifacts/observability/spool/successor-events.jsonl",
        ])
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
        .map_err(|_| capture_error("git-check-ignore-spawn-failed"))?;
    let deadline = Instant::now() + STATUS_TIMEOUT;
    loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|_| capture_error("git-check-ignore-wait-failed"))?
        {
            return match status.code() {
                Some(0) => runtime_store_untracked(&git, binding),
                Some(1) => Ok(false),
                _ => Err(capture_error("git-check-ignore-nonzero")),
            };
        }
        if Instant::now() >= deadline {
            terminate(&mut child);
            return Err(limit_error("git-check-ignore-time-limit-exceeded"));
        }
        thread::sleep(Duration::from_millis(2));
    }
}

fn runtime_store_untracked(
    git: &Path,
    binding: &RoutineBinding,
) -> Result<bool, RoutineError> {
    let mut child = Command::new(git)
        .args([
            "--no-optional-locks",
            "ls-files",
            "--error-unmatch",
            "--",
            "validation_artifacts/observability/spool/successor-events.jsonl",
        ])
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
        .map_err(|_| capture_error("git-ls-files-spawn-failed"))?;
    let deadline = Instant::now() + STATUS_TIMEOUT;
    loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|_| capture_error("git-ls-files-wait-failed"))?
        {
            return match status.code() {
                Some(1) => Ok(true),
                Some(0) => Ok(false),
                _ => Err(capture_error("git-ls-files-nonzero")),
            };
        }
        if Instant::now() >= deadline {
            terminate(&mut child);
            return Err(limit_error("git-ls-files-time-limit-exceeded"));
        }
        thread::sleep(Duration::from_millis(2));
    }
}

fn collect_bounded(child: &mut Child) -> Result<Vec<u8>, RoutineError> {
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| capture_error("git-status-pipe-missing"))?;
    let (sender, receiver) = mpsc::sync_channel(1);
    let reader = thread::spawn(move || {
        let _ = sender.send(read_limited(stdout));
    });
    let deadline = Instant::now() + STATUS_TIMEOUT;
    let mut output = None;
    let mut exit = None;
    while output.is_none() || exit.is_none() {
        receive_output(&receiver, &mut output, child)?;
        if exit.is_none() {
            exit = child
                .try_wait()
                .map_err(|_| capture_error("git-status-wait-failed"))?;
        }
        if output.as_ref().is_some_and(Result::is_err) {
            terminate(child);
            break;
        }
        if Instant::now() >= deadline {
            terminate(child);
            let _ = reader.join();
            return Err(limit_error("git-status-time-limit-exceeded"));
        }
        if output.is_none() || exit.is_none() {
            thread::sleep(Duration::from_millis(2));
        }
    }
    let _ = reader.join();
    let bytes = output.ok_or_else(|| capture_error("git-status-output-missing"))??;
    validate_exit(exit.ok_or_else(|| capture_error("git-status-exit-missing"))?)?;
    Ok(bytes)
}

fn receive_output(
    receiver: &mpsc::Receiver<Result<Vec<u8>, RoutineError>>,
    output: &mut Option<Result<Vec<u8>, RoutineError>>,
    child: &mut Child,
) -> Result<(), RoutineError> {
    if output.is_some() {
        return Ok(());
    }
    match receiver.try_recv() {
        Ok(result) => *output = Some(result),
        Err(TryRecvError::Disconnected) => {
            terminate(child);
            return Err(capture_error("git-status-reader-disconnected"));
        }
        Err(TryRecvError::Empty) => {}
    }
    Ok(())
}

fn terminate(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn read_limited(mut stdout: impl Read) -> Result<Vec<u8>, RoutineError> {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 16 * 1024];
    loop {
        let read = stdout
            .read(&mut buffer)
            .map_err(|_| capture_error("git-status-read-failed"))?;
        if read == 0 {
            break;
        }
        if bytes.len().saturating_add(read) > STATUS_LIMIT {
            return Err(limit_error("git-status-output-limit-exceeded"));
        }
        bytes.extend_from_slice(&buffer[..read]);
    }
    Ok(bytes)
}

fn validate_exit(status: ExitStatus) -> Result<(), RoutineError> {
    if status.success() {
        Ok(())
    } else {
        Err(capture_error("git-status-nonzero"))
    }
}

fn capability_error(cause: &'static str) -> RoutineError {
    RoutineError::new(RoutineErrorId::CapabilityUnavailable, cause, None)
}

fn capture_error(cause: &'static str) -> RoutineError {
    RoutineError::new(RoutineErrorId::CaptureFailed, cause, None)
}

fn limit_error(cause: &'static str) -> RoutineError {
    RoutineError::new(RoutineErrorId::CaptureLimit, cause, None)
}
