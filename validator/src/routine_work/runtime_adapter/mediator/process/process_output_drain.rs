use super::*;

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

#[cfg(test)]
pub(crate) type SetupFailureHook = Box<dyn FnOnce() + Send + 'static>;

#[cfg(test)]
pub(crate) fn setup_failure_hook() -> &'static Mutex<Option<(SetupFailurePoint, SetupFailureHook)>>
{
    static HOOK: OnceLock<Mutex<Option<(SetupFailurePoint, SetupFailureHook)>>> = OnceLock::new();
    HOOK.get_or_init(|| Mutex::new(None))
}

#[cfg(test)]
pub(crate) fn set_test_process_setup_failure(
    point: SetupFailurePoint,
    hook: impl FnOnce() + Send + 'static,
) {
    *setup_failure_hook()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some((point, Box::new(hook)));
}

#[cfg(test)]
pub(crate) fn maybe_inject_setup_failure(
    point: SetupFailurePoint,
    cause: &'static str,
) -> Result<(), RoutineError> {
    let hook = {
        let mut slot = setup_failure_hook()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if slot.as_ref().map(|(expected, _)| *expected) == Some(point) {
            slot.take().map(|(_, hook)| hook)
        } else {
            None
        }
    };
    if let Some(hook) = hook {
        hook();
        return Err(mediator_error(cause));
    }
    Ok(())
}

#[cfg(not(test))]
pub(crate) fn maybe_inject_setup_failure(
    _point: SetupFailurePoint,
    _cause: &'static str,
) -> Result<(), RoutineError> {
    Ok(())
}

#[cfg(target_os = "macos")]
pub(crate) fn sandbox_profile(
    program: &Path,
    working_directory: &Path,
    read_sources: &[&Path],
    scopes: &[&Path],
) -> Result<String, RoutineError> {
    let mut profile = String::from(
        "(version 1)\n(allow default)\n(deny network*)\n(deny process-fork (with send-signal SIGKILL))\n(deny process-exec)\n(deny file-map-executable)\n(allow file-map-executable (subpath \"/System\"))\n(allow file-map-executable (subpath \"/usr/lib\"))\n(deny file-read*)\n(allow file-read* (literal \"/\"))\n(allow file-read* (subpath \"/System\"))\n(allow file-read* (subpath \"/usr/lib\"))\n(allow file-read* (subpath \"/private/var/db/dyld\"))\n(deny file-write*)\n(deny file-clone file-link)\n",
    );
    let program = program
        .to_str()
        .ok_or_else(|| mediator_error("mediator-executable-path-not-utf8"))?;
    profile.push_str("(allow process-exec (literal \"");
    profile.push_str(&sandbox_escape(program)?);
    profile.push_str("\"))\n");
    for ancestor in working_directory
        .ancestors()
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
    {
        if ancestor == Path::new("/") {
            continue;
        }
        let text = ancestor
            .to_str()
            .ok_or_else(|| mediator_error("mediator-working-directory-not-utf8"))?;
        profile.push_str("(allow file-read-metadata (literal \"");
        profile.push_str(&sandbox_escape(text)?);
        profile.push_str("\"))\n");
    }
    profile.push_str("(allow file-read* (literal \"");
    profile.push_str(&sandbox_escape(program)?);
    profile.push_str("\"))\n");
    for source in read_sources {
        let text = source
            .to_str()
            .ok_or_else(|| mediator_error("mediator-read-source-path-not-utf8"))?;
        profile.push_str("(allow file-read* (literal \"");
        profile.push_str(&sandbox_escape(text)?);
        profile.push_str("\"))\n");
        if let Some(alias) = private_alias(source) {
            let alias = alias
                .to_str()
                .ok_or_else(|| mediator_error("mediator-read-source-path-not-utf8"))?;
            profile.push_str("(allow file-read* (literal \"");
            profile.push_str(&sandbox_escape(alias)?);
            profile.push_str("\"))\n");
        }
    }
    for scope in scopes {
        let text = scope
            .to_str()
            .ok_or_else(|| mediator_error("mediator-output-scope-not-utf8"))?;
        for ancestor in scope.ancestors().collect::<Vec<_>>().into_iter().rev() {
            if ancestor == Path::new("/") || ancestor == working_directory {
                continue;
            }
            let ancestor = ancestor
                .to_str()
                .ok_or_else(|| mediator_error("mediator-output-scope-not-utf8"))?;
            profile.push_str("(allow file-read-metadata (literal \"");
            profile.push_str(&sandbox_escape(ancestor)?);
            profile.push_str("\"))\n");
            if let Some(alias) = private_alias(Path::new(ancestor)) {
                let alias = alias
                    .to_str()
                    .ok_or_else(|| mediator_error("mediator-output-scope-not-utf8"))?;
                profile.push_str("(allow file-read-metadata (literal \"");
                profile.push_str(&sandbox_escape(alias)?);
                profile.push_str("\"))\n");
            }
        }
        profile.push_str("(allow file-read-metadata (subpath \"");
        profile.push_str(&sandbox_escape(text)?);
        profile.push_str("\"))\n");
        if let Some(alias) = private_alias(scope) {
            let alias = alias
                .to_str()
                .ok_or_else(|| mediator_error("mediator-output-scope-not-utf8"))?;
            profile.push_str("(allow file-read-metadata (subpath \"");
            profile.push_str(&sandbox_escape(alias)?);
            profile.push_str("\"))\n");
            profile.push_str("(allow file-write* (subpath \"");
            profile.push_str(&sandbox_escape(alias)?);
            profile.push_str("\"))\n");
        }
        profile.push_str("(allow file-write* (subpath \"");
        profile.push_str(&sandbox_escape(text)?);
        profile.push_str("\"))\n");
    }
    Ok(profile)
}

#[cfg(target_os = "macos")]
pub(crate) fn private_alias(path: &Path) -> Option<PathBuf> {
    path.strip_prefix("/private")
        .ok()
        .filter(|suffix| !suffix.as_os_str().is_empty())
        .map(|suffix| Path::new("/").join(suffix))
}

#[cfg(target_os = "macos")]
pub(crate) fn sandbox_escape(value: &str) -> Result<String, RoutineError> {
    if value.bytes().any(|byte| byte.is_ascii_control()) {
        return Err(mediator_error("mediator-sandbox-path-invalid"));
    }
    Ok(value.replace('\\', "\\\\").replace('"', "\\\""))
}

#[cfg(unix)]
pub(crate) fn status_kind(status: ExitStatus) -> ProcessTermination {
    status.code().map_or_else(
        || ProcessTermination::Signaled(status.signal().unwrap_or(0)),
        ProcessTermination::Exited,
    )
}
