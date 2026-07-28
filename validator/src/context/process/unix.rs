use super::bounded_capture::{Captured, MAX_CAPTURE_BYTES};
use super::group_custody::{
    drain_after_exit, group_exists, set_nonblocking, terminate, terminate_without_drain,
};
use super::{ProbeOutput, probe};
use crate::context::ContextError;
use std::ffi::OsString;
use std::io;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

pub(super) fn run(
    program: &Path,
    args: &[OsString],
    current_dir: &Path,
    timeout: Duration,
) -> Result<ProbeOutput, ContextError> {
    let label = program.display().to_string();
    let program_directory = program.parent().unwrap_or_else(|| Path::new("/usr/bin"));
    let mut command = Command::new(program);
    command
        .args(args)
        .current_dir(current_dir)
        .env_clear()
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .env("PATH", program_directory)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_ATTR_NOSYSTEM", "1")
        .env("GIT_NO_REPLACE_OBJECTS", "1")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_PAGER", "cat")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // SAFETY: this closure uses only the async-signal-safe setpgid syscall
    // between fork and exec.
    unsafe {
        command.pre_exec(|| {
            if libc::setpgid(0, 0) == 0 {
                Ok(())
            } else {
                Err(io::Error::last_os_error())
            }
        });
    }
    let mut child = command
        .spawn()
        .map_err(|error| probe(&label, error.to_string()))?;
    let group = i32::try_from(child.id())
        .map_err(|_| probe(&label, "child process-group identity is invalid"))?;
    let mut stdout = match child.stdout.take() {
        Some(stdout) => stdout,
        None => {
            return Err(cleanup_result(
                &label,
                "stdout pipe is unavailable",
                terminate_without_drain(&mut child, group),
            ));
        }
    };
    let mut stderr = match child.stderr.take() {
        Some(stderr) => stderr,
        None => {
            return Err(cleanup_result(
                &label,
                "stderr pipe is unavailable",
                terminate_without_drain(&mut child, group),
            ));
        }
    };
    if let Err(error) = set_nonblocking(&stdout) {
        return Err(cleanup_result(
            &label,
            &error.to_string(),
            terminate_without_drain(&mut child, group),
        ));
    }
    if let Err(error) = set_nonblocking(&stderr) {
        return Err(cleanup_result(
            &label,
            &error.to_string(),
            terminate_without_drain(&mut child, group),
        ));
    }

    let mut captured_stdout = Captured::new();
    let mut captured_stderr = Captured::new();
    let deadline = Instant::now() + timeout;
    loop {
        if let Err(error) = captured_stdout
            .drain(&mut stdout)
            .and_then(|()| captured_stderr.drain(&mut stderr))
        {
            return Err(cleanup_result(
                &label,
                &error.to_string(),
                terminate(&mut child, group, &mut stdout, &mut stderr),
            ));
        }
        if captured_stdout.overflow || captured_stderr.overflow {
            return Err(cleanup_result(
                &label,
                &format!("output exceeded {MAX_CAPTURE_BYTES} bytes"),
                terminate(&mut child, group, &mut stdout, &mut stderr),
            ));
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                let descendants = group_exists(group);
                if descendants {
                    return Err(probe(
                        &label,
                        "descendant custody became ambiguous after leader exit; reconciliation required",
                    ));
                }
                drain_after_exit(
                    &mut stdout,
                    &mut stderr,
                    &mut captured_stdout,
                    &mut captured_stderr,
                )
                .map_err(|error| probe(&label, error.to_string()))?;
                if !captured_stdout.eof || !captured_stderr.eof {
                    return Err(probe(
                        &label,
                        "output pipes remained open after process exit",
                    ));
                }
                if captured_stdout.overflow || captured_stderr.overflow {
                    return Err(output_overflow(&label));
                }
                return Ok(output(status, captured_stdout, captured_stderr));
            }
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(2));
            }
            Ok(None) => {
                return Err(cleanup_result(
                    &label,
                    &format!("timed out after {} ms", timeout.as_millis()),
                    terminate(&mut child, group, &mut stdout, &mut stderr),
                ));
            }
            Err(error) => {
                return Err(cleanup_result(
                    &label,
                    &error.to_string(),
                    terminate(&mut child, group, &mut stdout, &mut stderr),
                ));
            }
        }
    }
}

fn output(status: ExitStatus, stdout: Captured, stderr: Captured) -> ProbeOutput {
    ProbeOutput {
        stdout: stdout.bytes,
        stderr: stderr.bytes,
        success: status.success(),
    }
}

fn output_overflow(program: &str) -> ContextError {
    probe(
        program,
        format!("output exceeded {MAX_CAPTURE_BYTES} bytes"),
    )
}

fn cleanup_result(program: &str, primary: &str, cleanup: std::io::Result<()>) -> ContextError {
    match cleanup {
        Ok(()) => probe(program, primary),
        Err(error) => probe(
            program,
            format!("{primary}; cleanup failed: {error}; reconciliation required"),
        ),
    }
}
