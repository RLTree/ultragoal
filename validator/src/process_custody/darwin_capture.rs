use std::io::{self, Read, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};

pub(crate) struct DarwinCapturedOutput {
    pub(crate) retained: Vec<u8>,
    pub(crate) digest: String,
    pub(crate) bytes: u64,
    pub(crate) overflow: bool,
}

pub(crate) struct DarwinCaptureHandles {
    stdout: Option<JoinHandle<io::Result<DarwinCapturedOutput>>>,
    stderr: Option<JoinHandle<io::Result<DarwinCapturedOutput>>>,
    stdin: Option<JoinHandle<io::Result<()>>>,
    stdout_overflow: Arc<AtomicBool>,
    stderr_overflow: Arc<AtomicBool>,
}

pub(crate) struct DarwinCaptureStartFailure {
    pub(crate) handles: DarwinCaptureHandles,
}

pub(crate) struct DarwinCaptureJoinFailure {
    pub(crate) handles: DarwinCaptureHandles,
}

impl DarwinCaptureHandles {
    pub(crate) fn overflowed(&self) -> bool {
        self.stdout_overflow.load(Ordering::Acquire) || self.stderr_overflow.load(Ordering::Acquire)
    }
}

pub(crate) fn start_capture(
    stdin: std::fs::File,
    stdout: std::fs::File,
    stderr: std::fs::File,
    input: &[u8],
    stdout_limit: usize,
    stderr_limit: usize,
) -> Result<DarwinCaptureHandles, DarwinCaptureStartFailure> {
    let stdout_overflow = Arc::new(AtomicBool::new(false));
    let stderr_overflow = Arc::new(AtomicBool::new(false));
    let stdout_overflow_thread = Arc::clone(&stdout_overflow);
    let stdout = thread::Builder::new()
        .name("harness-darwin-stdout".to_owned())
        .spawn(move || read_bounded(stdout, stdout_limit, true, stdout_overflow_thread));
    let stdout = match stdout {
        Ok(handle) => Some(handle),
        Err(_) => {
            return Err(DarwinCaptureStartFailure {
                handles: DarwinCaptureHandles {
                    stdout: None,
                    stderr: None,
                    stdin: None,
                    stdout_overflow,
                    stderr_overflow,
                },
            });
        }
    };
    let stderr_overflow_thread = Arc::clone(&stderr_overflow);
    let stderr = match thread::Builder::new()
        .name("harness-darwin-stderr".to_owned())
        .spawn(move || read_bounded(stderr, stderr_limit, true, stderr_overflow_thread))
    {
        Ok(handle) => Some(handle),
        Err(_) => {
            return Err(DarwinCaptureStartFailure {
                handles: DarwinCaptureHandles {
                    stdout,
                    stderr: None,
                    stdin: None,
                    stdout_overflow,
                    stderr_overflow,
                },
            });
        }
    };
    let input = input.to_vec();
    let stdin = match thread::Builder::new()
        .name("harness-darwin-stdin".to_owned())
        .spawn(move || {
            let mut stdin = stdin;
            stdin.write_all(&input)?;
            stdin.flush()
        }) {
        Ok(handle) => Some(handle),
        Err(_) => {
            return Err(DarwinCaptureStartFailure {
                handles: DarwinCaptureHandles {
                    stdout,
                    stderr,
                    stdin: None,
                    stdout_overflow,
                    stderr_overflow,
                },
            });
        }
    };
    Ok(DarwinCaptureHandles {
        stdout,
        stderr,
        stdin,
        stdout_overflow,
        stderr_overflow,
    })
}

pub(crate) fn join_capture(
    mut handles: DarwinCaptureHandles,
) -> Result<(DarwinCapturedOutput, DarwinCapturedOutput), DarwinCaptureJoinFailure> {
    let stdout = match join_output(handles.stdout.take(), "stdout") {
        Ok(output) => output,
        Err(_) => return Err(DarwinCaptureJoinFailure { handles }),
    };
    let stderr = match join_output(handles.stderr.take(), "stderr") {
        Ok(output) => output,
        Err(_) => return Err(DarwinCaptureJoinFailure { handles }),
    };
    if join_stdin(handles.stdin.take()).is_err() {
        return Err(DarwinCaptureJoinFailure { handles });
    }
    Ok((stdout, stderr))
}

pub(crate) fn discard_after_cleanup(mut handles: DarwinCaptureHandles) -> io::Result<()> {
    let mut failure = None;
    if let Some(handle) = handles.stdout.take() {
        if let Err(error) = join_output(Some(handle), "stdout") {
            failure = Some(error);
        }
    }
    if let Some(handle) = handles.stderr.take() {
        if let Err(error) = join_output(Some(handle), "stderr") {
            failure.get_or_insert(error);
        }
    }
    if let Some(handle) = handles.stdin.take() {
        if let Err(error) = join_stdin(Some(handle)) {
            failure.get_or_insert(error);
        }
    }
    failure.map_or(Ok(()), Err)
}

fn join_output(
    handle: Option<JoinHandle<io::Result<DarwinCapturedOutput>>>,
    stream: &str,
) -> io::Result<DarwinCapturedOutput> {
    handle
        .ok_or_else(|| io::Error::other(format!("{stream} capture thread missing")))?
        .join()
        .map_err(|_| io::Error::other(format!("{stream} capture thread panicked")))?
}

fn join_stdin(handle: Option<JoinHandle<io::Result<()>>>) -> io::Result<()> {
    handle
        .ok_or_else(|| io::Error::other("stdin writer thread missing"))?
        .join()
        .map_err(|_| io::Error::other("stdin writer thread panicked"))?
}

fn read_bounded(
    mut reader: std::fs::File,
    limit: usize,
    retain: bool,
    overflow_flag: Arc<AtomicBool>,
) -> io::Result<DarwinCapturedOutput> {
    use sha2::{Digest, Sha256};
    let mut retained = Vec::new();
    let mut hasher = Sha256::new();
    let mut bytes = 0_u64;
    let mut overflow = false;
    let mut buffer = [0_u8; 16 * 1024];
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            return Ok(DarwinCapturedOutput {
                retained,
                digest: format!("sha256:{:x}", hasher.finalize()),
                bytes,
                overflow,
            });
        }
        hasher.update(&buffer[..count]);
        bytes = bytes.saturating_add(count as u64);
        if retain {
            let accepted = limit.saturating_sub(retained.len()).min(count);
            retained.extend_from_slice(&buffer[..accepted]);
        }
        if bytes > limit as u64 {
            overflow = true;
            overflow_flag.store(true, Ordering::Release);
        }
    }
}
