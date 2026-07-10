use super::error::ContextError;
use std::ffi::OsString;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const MAX_CAPTURE_BYTES: usize = 64 * 1024 * 1024;

pub(crate) struct ProbeOutput {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub success: bool,
}

struct Captured {
    bytes: Vec<u8>,
    truncated: bool,
}

fn drain(mut reader: impl Read) -> Captured {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 16 * 1024];
    let mut truncated = false;
    loop {
        let read = match reader.read(&mut buffer) {
            Ok(0) | Err(_) => break,
            Ok(read) => read,
        };
        let remaining = MAX_CAPTURE_BYTES.saturating_sub(bytes.len());
        let retained = remaining.min(read);
        bytes.extend_from_slice(&buffer[..retained]);
        truncated |= retained != read;
    }
    Captured { bytes, truncated }
}

pub(crate) fn run_bounded(
    program: &Path,
    args: &[OsString],
    current_dir: &Path,
    timeout: Duration,
) -> Result<ProbeOutput, ContextError> {
    let output = run_bounded_allow_failure(program, args, current_dir, timeout)?;
    if !output.success {
        let message = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(ContextError::Probe {
            program: program.display().to_string(),
            message: if message.is_empty() {
                "non-zero exit".to_owned()
            } else {
                message
            },
        });
    }
    Ok(output)
}

pub(crate) fn run_bounded_allow_failure(
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
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_PAGER", "cat")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().map_err(|error| ContextError::Probe {
        program: label.clone(),
        message: error.to_string(),
    })?;
    let stdout = child.stdout.take().expect("piped stdout");
    let stderr = child.stderr.take().expect("piped stderr");
    let stdout_reader = thread::spawn(move || drain(stdout));
    let stderr_reader = thread::spawn(move || drain(stderr));
    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(5)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = stdout_reader.join();
                let _ = stderr_reader.join();
                return Err(ContextError::Probe {
                    program: label,
                    message: format!("timed out after {} ms", timeout.as_millis()),
                });
            }
            Err(error) => {
                return Err(ContextError::Probe {
                    program: label,
                    message: error.to_string(),
                });
            }
        }
    };
    let stdout = stdout_reader.join().map_err(|_| ContextError::Probe {
        program: label.clone(),
        message: "stdout reader panicked".to_owned(),
    })?;
    let stderr = stderr_reader.join().map_err(|_| ContextError::Probe {
        program: label.clone(),
        message: "stderr reader panicked".to_owned(),
    })?;
    if stdout.truncated || stderr.truncated {
        return Err(ContextError::Probe {
            program: label,
            message: format!("output exceeded {MAX_CAPTURE_BYTES} bytes"),
        });
    }
    Ok(ProbeOutput {
        stdout: stdout.bytes,
        stderr: stderr.bytes,
        success: status.success(),
    })
}
