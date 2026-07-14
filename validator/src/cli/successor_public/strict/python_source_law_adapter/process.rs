use super::contract::{
    PythonSourceLawAdapterError, PythonSourceLawRequest, PythonSourceLawResponse,
};
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const TIMEOUT: Duration = Duration::from_secs(60);
const MAX_OUTPUT_BYTES: usize = 2 * 1024 * 1024;

pub(in crate::cli::successor_public::strict) fn run(
    request: PythonSourceLawRequest,
) -> Result<PythonSourceLawResponse, PythonSourceLawAdapterError> {
    verify_identity(&request.executable, &request.executable_sha256)?;
    let script = request.root.join("scripts/check-python-source-laws");
    if !script.is_file() || script.is_symlink() {
        return Err(PythonSourceLawAdapterError::ScriptUnavailable);
    }
    let mut command = Command::new(&request.executable);
    command
        .arg(&script)
        .arg(&request.root)
        .current_dir(&request.root)
        .env_clear()
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|_| PythonSourceLawAdapterError::SpawnFailed)?;
    let stdout = child
        .stdout
        .take()
        .ok_or(PythonSourceLawAdapterError::SpawnFailed)?;
    let stderr = child
        .stderr
        .take()
        .ok_or(PythonSourceLawAdapterError::SpawnFailed)?;
    let stdout_reader = thread::spawn(move || bounded_read(stdout));
    let stderr_reader = thread::spawn(move || bounded_read(stderr));
    let deadline = Instant::now() + TIMEOUT;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(5)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(PythonSourceLawAdapterError::TimedOut);
            }
            Err(_) => return Err(PythonSourceLawAdapterError::SpawnFailed),
        }
    };
    let stdout = stdout_reader
        .join()
        .map_err(|_| PythonSourceLawAdapterError::OutputInvalid)??;
    let stderr = stderr_reader
        .join()
        .map_err(|_| PythonSourceLawAdapterError::OutputInvalid)??;
    verify_identity(&request.executable, &request.executable_sha256)?;
    match status.code() {
        Some(0) => super::diagnostic::parse_pass(&stdout, &stderr),
        Some(1) => super::diagnostic::parse_findings(&stdout, &stderr),
        _ => Err(PythonSourceLawAdapterError::UnexpectedExit),
    }
}

fn verify_identity(path: &Path, expected_sha256: &str) -> Result<(), PythonSourceLawAdapterError> {
    let actual =
        crate::digest::file(path).map_err(|_| PythonSourceLawAdapterError::IdentityMismatch)?;
    if actual == format!("sha256:{expected_sha256}") {
        Ok(())
    } else {
        Err(PythonSourceLawAdapterError::IdentityMismatch)
    }
}

fn bounded_read(mut input: impl Read) -> Result<Vec<u8>, PythonSourceLawAdapterError> {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 16 * 1024];
    loop {
        let read = input
            .read(&mut buffer)
            .map_err(|_| PythonSourceLawAdapterError::OutputInvalid)?;
        if read == 0 {
            return Ok(bytes);
        }
        if bytes.len().saturating_add(read) > MAX_OUTPUT_BYTES {
            return Err(PythonSourceLawAdapterError::OutputTooLarge);
        }
        bytes.extend_from_slice(&buffer[..read]);
    }
}
