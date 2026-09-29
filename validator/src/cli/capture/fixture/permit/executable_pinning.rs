use super::*;

#[cfg(unix)]
pub(crate) fn pin_source_executable(
    path: &Path,
    issuance_policy: ExecutableIssuancePolicy,
) -> Result<(Vec<u8>, String, ExecutableIdentity, PinnedExecutableKind), FixtureScheduleError> {
    let path_before = std::fs::symlink_metadata(path)?;
    require_safe_source(&path_before)?;
    let file = open_source_descriptor(path)?;
    let opened = file.metadata()?;
    require_safe_source(&opened)?;
    let identity = ExecutableIdentity::from(&opened);
    if ExecutableIdentity::from(&path_before) != identity {
        return Err(FixtureScheduleError::Integrity(
            "fixture executable changed while it was opened".to_owned(),
        ));
    }
    let bytes = read_exact_descriptor(&file, MAX_EXECUTABLE_BYTES)?;
    test_issue_pause(path);
    let descriptor_after = file.metadata()?;
    let path_after = std::fs::symlink_metadata(path)?;
    require_safe_source(&descriptor_after)?;
    require_safe_source(&path_after)?;
    if ExecutableIdentity::from(&descriptor_after) != identity
        || ExecutableIdentity::from(&path_after) != identity
    {
        return Err(FixtureScheduleError::Integrity(
            "fixture executable changed during permit issuance".to_owned(),
        ));
    }
    let digest = crate::digest::bytes(&bytes);
    let kind = classify_executable(path, &bytes, identity, issuance_policy)?;
    Ok((bytes, digest, identity, kind))
}

#[cfg(unix)]
pub(crate) fn classify_executable(
    path: &Path,
    bytes: &[u8],
    identity: ExecutableIdentity,
    _issuance_policy: ExecutableIssuancePolicy,
) -> Result<PinnedExecutableKind, FixtureScheduleError> {
    let native_magic = bytes.get(..4).is_some_and(|magic| {
        matches!(
            magic,
            b"\x7fELF"
                | b"\xfe\xed\xfa\xce"
                | b"\xfe\xed\xfa\xcf"
                | b"\xce\xfa\xed\xfe"
                | b"\xcf\xfa\xed\xfe"
                | b"\xca\xfe\xba\xbe"
                | b"\xca\xfe\xba\xbf"
        )
    });
    if native_magic && identity.user_id == 0 {
        if _issuance_policy == ExecutableIssuancePolicy::ShellSubstrate {
            require_protected_path(path, &[POSIX_SHELL_PATH])?;
        } else {
            require_protected_native_path(path)?;
        }
        return Ok(PinnedExecutableKind::ProtectedNative);
    }
    #[cfg(test)]
    if native_magic && _issuance_policy == ExecutableIssuancePolicy::TestNativeSnapshot {
        return Ok(PinnedExecutableKind::TestNativeSnapshot);
    }
    if bytes.len() <= MAX_SHELL_SOURCE_BYTES
        && bytes.starts_with(b"#!/bin/sh\n")
        && !bytes.contains(&0)
    {
        return Ok(PinnedExecutableKind::PosixShellSource);
    }
    Err(FixtureScheduleError::Integrity(
        "fixture executable must be a protected native binary or exact POSIX shell source"
            .to_owned(),
    ))
}

#[cfg(unix)]
pub(crate) fn require_protected_native_path(path: &Path) -> Result<(), FixtureScheduleError> {
    require_protected_path(path, &PROTECTED_NATIVE_PATHS)
}

#[cfg(unix)]
pub(crate) fn require_protected_path(
    path: &Path,
    allowed_paths: &[&str],
) -> Result<(), FixtureScheduleError> {
    use std::os::unix::fs::MetadataExt;

    if !allowed_paths
        .iter()
        .any(|allowed| path == Path::new(allowed))
    {
        return Err(FixtureScheduleError::Integrity(
            "fixture native executable is not a fixed protected substrate".to_owned(),
        ));
    }

    let mut component = Some(path);
    while let Some(current) = component {
        let metadata = std::fs::symlink_metadata(current)?;
        if metadata.file_type().is_symlink()
            || metadata.uid() != 0
            || metadata.mode() & 0o022 != 0
            || (current == path
                && (!metadata.is_file() || metadata.nlink() != 1 || metadata.mode() & 0o111 == 0))
            || (current != path && !metadata.is_dir())
        {
            return Err(FixtureScheduleError::Integrity(
                "fixture native executable path authority is unsafe".to_owned(),
            ));
        }
        component = current.parent().filter(|parent| *parent != current);
    }
    Ok(())
}

#[cfg(unix)]
pub(crate) fn open_source_descriptor(path: &Path) -> Result<std::fs::File, FixtureScheduleError> {
    use std::ffi::CString;
    use std::os::fd::FromRawFd;
    use std::os::unix::ffi::OsStrExt;

    let path = CString::new(path.as_os_str().as_bytes()).map_err(|_| {
        FixtureScheduleError::InvalidMetadata("fixture executable path contains NUL".to_owned())
    })?;
    // SAFETY: `path` is a NUL-terminated CString retained for the call and the
    // fixed flags prevent link traversal and descriptor inheritance.
    let descriptor = unsafe {
        libc::open(
            path.as_ptr(),
            libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK,
        )
    };
    if descriptor < 0 {
        return Err(FixtureScheduleError::Io(std::io::Error::last_os_error()));
    }
    // SAFETY: `open` returned a new owned nonnegative descriptor.
    Ok(unsafe { std::fs::File::from_raw_fd(descriptor) })
}

#[cfg(unix)]
pub(crate) fn require_safe_source(
    metadata: &std::fs::Metadata,
) -> Result<(), FixtureScheduleError> {
    use std::os::unix::fs::MetadataExt;

    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.nlink() != 1
        || metadata.len() == 0
        || metadata.len() > MAX_EXECUTABLE_BYTES as u64
        || metadata.mode() & 0o022 != 0
        || metadata.mode() & 0o111 == 0
    {
        return Err(FixtureScheduleError::Integrity(
            "fixture executable identity is unsafe".to_owned(),
        ));
    }
    Ok(())
}

#[cfg(unix)]
pub(crate) fn read_exact_descriptor(
    file: &std::fs::File,
    maximum_bytes: usize,
) -> Result<Vec<u8>, FixtureScheduleError> {
    use std::os::unix::fs::FileExt;

    let before = file.metadata()?;
    if before.len() == 0 || before.len() > maximum_bytes as u64 {
        return Err(FixtureScheduleError::Integrity(
            "fixture executable size is out of bounds".to_owned(),
        ));
    }
    let identity = ExecutableIdentity::from(&before);
    let mut bytes = vec![0_u8; before.len() as usize];
    let mut offset = 0_usize;
    while offset < bytes.len() {
        let read = file.read_at(&mut bytes[offset..], offset as u64)?;
        if read == 0 {
            return Err(FixtureScheduleError::Integrity(
                "fixture executable shortened during snapshot".to_owned(),
            ));
        }
        offset += read;
    }
    if ExecutableIdentity::from(&file.metadata()?) != identity {
        return Err(FixtureScheduleError::Integrity(
            "fixture executable changed during snapshot".to_owned(),
        ));
    }
    Ok(bytes)
}

#[cfg(test)]
pub(crate) static TEST_ISSUE_PAUSED: AtomicBool = AtomicBool::new(false);

#[cfg(test)]
pub(crate) fn issue_hook() -> &'static std::sync::Mutex<Option<(PathBuf, u64)>> {
    static HOOK: std::sync::OnceLock<std::sync::Mutex<Option<(PathBuf, u64)>>> =
        std::sync::OnceLock::new();
    HOOK.get_or_init(|| std::sync::Mutex::new(None))
}

#[cfg(test)]
pub(crate) fn test_issue_pause(path: &Path) {
    let milliseconds = {
        let mut hook = issue_hook().lock().expect("fixture issue hook lock");
        if hook.as_ref().is_some_and(|(target, _)| target == path) {
            hook.take().map(|(_, milliseconds)| milliseconds)
        } else {
            None
        }
    };
    if let Some(milliseconds) = milliseconds {
        TEST_ISSUE_PAUSED.store(true, Ordering::SeqCst);
        std::thread::sleep(std::time::Duration::from_millis(milliseconds));
    }
}

#[cfg(not(test))]
pub(crate) fn test_issue_pause(_path: &Path) {}
