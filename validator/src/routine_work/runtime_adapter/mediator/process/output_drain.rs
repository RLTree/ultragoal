use super::*;

pub(crate) struct Drained {
    pub(crate) retained: Vec<u8>,
    pub(crate) sha256: String,
    pub(crate) closed: bool,
}

pub(crate) fn drain(
    mut reader: impl Read,
    limit: u64,
    observed: &AtomicU64,
    overflow: &AtomicBool,
    done: &AtomicBool,
    retain: bool,
) -> Result<Drained, RoutineError> {
    let mut retained = Vec::new();
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 16 * 1024];
    loop {
        if done.load(Ordering::Acquire) && overflow.load(Ordering::Acquire) {
            return Ok(Drained {
                retained,
                sha256: format!("sha256:{:x}", hasher.finalize()),
                closed: false,
            });
        }
        let read = match reader.read(&mut buffer) {
            Ok(read) => read,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                if done.load(Ordering::Acquire) {
                    return Ok(Drained {
                        retained,
                        sha256: format!("sha256:{:x}", hasher.finalize()),
                        closed: false,
                    });
                }
                std::thread::sleep(Duration::from_millis(2));
                continue;
            }
            Err(_) => return Err(mediator_error("mediator-output-read-failed")),
        };
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        let prior = observed.fetch_add(read as u64, Ordering::AcqRel);
        let remaining = limit.saturating_sub(prior);
        let accepted = remaining.min(read as u64) as usize;
        if retain {
            retained.extend_from_slice(&buffer[..accepted]);
        }
        if accepted != read {
            overflow.store(true, Ordering::Release);
        }
    }
    Ok(Drained {
        retained,
        sha256: format!("sha256:{:x}", hasher.finalize()),
        closed: true,
    })
}

#[cfg(unix)]
pub(crate) fn set_nonblocking(file: &impl AsRawFd) -> Result<(), RoutineError> {
    let descriptor = file.as_raw_fd();
    let flags = unsafe { libc::fcntl(descriptor, libc::F_GETFL) };
    if flags < 0 || unsafe { libc::fcntl(descriptor, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0
    {
        return Err(mediator_error("mediator-output-nonblocking-failed"));
    }
    Ok(())
}
