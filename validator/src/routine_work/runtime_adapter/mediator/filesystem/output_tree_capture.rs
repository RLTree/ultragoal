use super::*;

#[cfg(unix)]
pub(crate) fn capture_tree(
    directory: &File,
    relative: &RepoPath,
    device: u64,
    budget: u64,
    consumed: &mut u64,
    files: &mut BTreeMap<String, OutputFileRecord>,
) -> Result<(), RoutineError> {
    let directory_before = ObjectIdentity::from(
        &directory
            .metadata()
            .map_err(|_| mediator_error("mediator-output-metadata-failed"))?,
    );
    let entries = directory_names(directory)?;
    run_test_capture_hook();
    for name in entries {
        if files.len() >= MAX_OUTPUT_FILES {
            return Err(mediator_error("mediator-output-file-limit-exceeded"));
        }
        let child_relative = RepoPath::parse(format!("{}/{}", relative.as_str(), name))?;
        let encoded = CString::new(name.as_bytes())
            .map_err(|_| mediator_error("mediator-output-path-invalid"))?;
        let mut child = open_child(directory, &encoded)?;
        let metadata = child
            .metadata()
            .map_err(|_| mediator_error("mediator-output-metadata-failed"))?;
        if (!metadata.is_file() && !metadata.is_dir()) || metadata.dev() != device {
            return Err(mediator_error("mediator-output-object-unsafe"));
        }
        let identity = ObjectIdentity::from(&metadata);
        if metadata.is_dir() {
            capture_tree(&child, &child_relative, device, budget, consumed, files)?;
            require_current_child(directory, &encoded, identity)?;
            continue;
        }
        if metadata.nlink() != 1 {
            return Err(mediator_error("mediator-output-hardlink-refused"));
        }
        *consumed = consumed
            .checked_add(metadata.len())
            .filter(|total| *total <= budget)
            .ok_or_else(|| mediator_error("mediator-output-artifact-budget-exceeded"))?;
        let digest = digest_reader(&mut child, metadata.len())?;
        let after = ObjectIdentity::from(
            &child
                .metadata()
                .map_err(|_| mediator_error("mediator-output-metadata-failed"))?,
        );
        if identity != after {
            return Err(RoutineError::new(
                RoutineErrorId::ConcurrentMutation,
                "mediator-output-replaced-during-capture",
                None,
            ));
        }
        require_current_child(directory, &encoded, identity)?;
        files.insert(
            child_relative.as_str().to_owned(),
            OutputFileRecord {
                sha256: digest,
                byte_length: metadata.len(),
                unix_mode: Some(metadata.mode()),
            },
        );
    }
    let directory_after = ObjectIdentity::from(
        &directory
            .metadata()
            .map_err(|_| mediator_error("mediator-output-metadata-failed"))?,
    );
    if directory_before != directory_after {
        return Err(RoutineError::new(
            RoutineErrorId::ConcurrentMutation,
            "mediator-output-directory-mutated-during-capture",
            None,
        ));
    }
    Ok(())
}

#[cfg(unix)]
pub(crate) fn open_child(directory: &File, name: &CStr) -> Result<File, RoutineError> {
    let descriptor = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_NONBLOCK | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if descriptor < 0 {
        return Err(mediator_error("mediator-output-object-unsafe"));
    }
    Ok(unsafe { File::from_raw_fd(descriptor) })
}

#[cfg(unix)]
pub(crate) fn require_current_child(
    directory: &File,
    name: &CStr,
    expected: ObjectIdentity,
) -> Result<(), RoutineError> {
    let current = open_child(directory, name).map_err(|_| {
        RoutineError::new(
            RoutineErrorId::ConcurrentMutation,
            "mediator-output-replaced-during-capture",
            None,
        )
    })?;
    let current = ObjectIdentity::from(
        &current
            .metadata()
            .map_err(|_| mediator_error("mediator-output-metadata-failed"))?,
    );
    if current != expected {
        return Err(RoutineError::new(
            RoutineErrorId::ConcurrentMutation,
            "mediator-output-replaced-during-capture",
            None,
        ));
    }
    Ok(())
}

#[cfg(unix)]
pub(crate) fn directory_names(directory: &File) -> Result<Vec<String>, RoutineError> {
    let dot = c".";
    let descriptor = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            dot.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if descriptor < 0 {
        return Err(mediator_error("mediator-output-directory-open-failed"));
    }
    let stream = unsafe { libc::fdopendir(descriptor) };
    if stream.is_null() {
        unsafe {
            libc::close(descriptor);
        }
        return Err(mediator_error("mediator-output-directory-read-failed"));
    }
    let stream = DirectoryStream(stream);
    let mut names = Vec::new();
    loop {
        clear_readdir_error();
        let entry = unsafe { libc::readdir(stream.0) };
        if entry.is_null() {
            if readdir_failed() {
                return Err(mediator_error("mediator-output-directory-read-failed"));
            }
            break;
        }
        let name = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) };
        let bytes = name.to_bytes();
        if bytes == b"." || bytes == b".." {
            continue;
        }
        let name = std::str::from_utf8(bytes)
            .map_err(|_| mediator_error("mediator-output-path-not-utf8"))?;
        names.push(name.to_owned());
    }
    names.sort();
    Ok(names)
}

#[cfg(unix)]
pub(crate) fn clear_readdir_error() {
    #[cfg(target_os = "macos")]
    unsafe {
        *libc::__error() = 0;
    }
    #[cfg(any(target_os = "linux", target_os = "android"))]
    unsafe {
        *libc::__errno_location() = 0;
    }
}
