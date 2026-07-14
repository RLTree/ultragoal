#[cfg(unix)]
fn capture_and_remove(
    parent_fd: RawFd,
    name: &CStr,
    expected_fd: RawFd,
    expected: EntryIdentity,
    guard: RootGuard<'_>,
) -> io::Result<()> {
    guard.validate()?;
    require_entry_identity(parent_fd, name, expected)?;
    run_before_capture_hook(name);

    #[cfg(target_os = "freebsd")]
    {
        // FreeBSD can condition the namespace removal on the already-open
        // descriptor in one syscall. Revalidate after the pre-capture hook,
        // then expose the final race window: funlinkat must reject any name
        // replacement without deleting either identity.
        guard.validate()?;
        require_entry_identity(parent_fd, name, expected)?;
        run_before_capture_hook(name);
        return identity_conditioned_unlink(parent_fd, name, expected_fd, expected.kind);
    }

    #[cfg(not(target_os = "freebsd"))]
    let quarantine = unused_quarantine_name(parent_fd)?;
    #[cfg(not(target_os = "freebsd"))]
    rename_noreplace(parent_fd, name, parent_fd, &quarantine)?;

    #[cfg(not(target_os = "freebsd"))]
    if require_entry_identity(parent_fd, &quarantine, expected).is_err() {
        restore_captured_entry(parent_fd, name, &quarantine, expected);
        return Err(io::Error::other("captured lease entry identity changed"));
    }

    // The destructive boundary must bind the name and the already-pinned
    // identity in one kernel operation. A final fstatat followed by unlinkat
    // is still replaceable between syscalls and can erase an unrelated entry.
    #[cfg(not(target_os = "freebsd"))]
    run_before_capture_hook(&quarantine);
    #[cfg(not(target_os = "freebsd"))]
    if let Err(error) =
        identity_conditioned_unlink(parent_fd, &quarantine, expected_fd, expected.kind)
    {
        // Restoration is non-destructive and is attempted only while the
        // captured name still denotes the expected identity. If an attacker
        // replaced it, retain both names for explicit recovery.
        restore_captured_entry(parent_fd, name, &quarantine, expected);
        return Err(error);
    }
    #[cfg(not(target_os = "freebsd"))]
    Ok(())
}

#[cfg(unix)]
fn restore_captured_entry(
    parent_fd: RawFd,
    original: &CStr,
    captured: &CStr,
    expected: EntryIdentity,
) {
    if matches!(entry_identity(parent_fd, original), Ok(None))
        && matches!(entry_identity(parent_fd, captured), Ok(Some(observed)) if observed == expected)
    {
        let _ = rename_noreplace(parent_fd, captured, parent_fd, original);
    }
}

#[cfg(target_os = "freebsd")]
fn identity_conditioned_unlink(
    parent_fd: RawFd,
    name: &CStr,
    expected_fd: RawFd,
    kind: EntryKind,
) -> io::Result<()> {
    unsafe extern "C" {
        fn funlinkat(
            directory_fd: libc::c_int,
            path: *const libc::c_char,
            file_fd: libc::c_int,
            flags: libc::c_int,
        ) -> libc::c_int;
    }

    let flags = if kind == EntryKind::Directory {
        libc::AT_REMOVEDIR
    } else {
        0
    };
    if unsafe { funlinkat(parent_fd, name.as_ptr(), expected_fd, flags) } == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(all(unix, not(target_os = "freebsd")))]
fn identity_conditioned_unlink(
    _parent_fd: RawFd,
    _name: &CStr,
    _expected_fd: RawFd,
    _kind: EntryKind,
) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "identity-conditioned filesystem deletion is unavailable on this platform; lease retained for recovery",
    ))
}

#[cfg(unix)]
fn unused_quarantine_name(parent_fd: RawFd) -> io::Result<CString> {
    for _ in 0..16 {
        let mut random = [0_u8; 16];
        fill_random(&mut random)?;
        let mut value = String::from(".hul-delete-");
        for byte in random {
            use std::fmt::Write as _;
            write!(&mut value, "{byte:02x}").expect("writing into String cannot fail");
        }
        let value = CString::new(value).expect("hex cleanup name contains no NUL");
        if entry_identity(parent_fd, &value)?.is_none() {
            return Ok(value);
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not allocate private cleanup name",
    ))
}

#[cfg(target_os = "macos")]
fn fill_random(output: &mut [u8]) -> io::Result<()> {
    unsafe { libc::arc4random_buf(output.as_mut_ptr().cast(), output.len()) };
    Ok(())
}

#[cfg(target_os = "linux")]
fn fill_random(output: &mut [u8]) -> io::Result<()> {
    let written = unsafe { libc::getrandom(output.as_mut_ptr().cast(), output.len(), 0) };
    if written == output.len() as isize {
        Ok(())
    } else if written < 0 {
        Err(io::Error::last_os_error())
    } else {
        Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "short operating-system random read",
        ))
    }
}

#[cfg(all(unix, not(any(target_os = "macos", target_os = "linux"))))]
fn fill_random(_output: &mut [u8]) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "secure cleanup nonce unavailable on this platform",
    ))
}

#[cfg(target_os = "macos")]
fn rename_noreplace(from_fd: RawFd, from: &CStr, to_fd: RawFd, to: &CStr) -> io::Result<()> {
    let result = unsafe {
        libc::renameatx_np(
            from_fd,
            from.as_ptr(),
            to_fd,
            to.as_ptr(),
            libc::RENAME_EXCL,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}
