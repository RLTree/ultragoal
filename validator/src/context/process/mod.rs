use std::ffi::OsString;
use std::path::Path;
use std::time::Duration;

use super::error::ContextError;

#[cfg(unix)]
mod bounded_capture;
#[cfg(unix)]
mod group_custody;
#[cfg(all(test, unix))]
mod tests;
#[cfg(unix)]
mod unix;

pub(crate) struct ProbeOutput {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub success: bool,
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
    #[cfg(not(unix))]
    {
        let _ = (program, args, current_dir, timeout);
        return Err(ContextError::Probe {
            program: "process custody".to_owned(),
            message: "bounded process-group custody is unavailable on this platform".to_owned(),
        });
    }

    #[cfg(unix)]
    unix::run(program, args, current_dir, timeout)
}

fn probe(program: &str, message: impl Into<String>) -> ContextError {
    ContextError::Probe {
        program: program.to_owned(),
        message: message.into(),
    }
}
