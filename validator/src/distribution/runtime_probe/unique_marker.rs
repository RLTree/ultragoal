fn unique_marker(stdout: &[u8]) -> Result<&[u8], DistributionError> {
    let positions = stdout
        .windows(MARKER.len())
        .enumerate()
        .filter_map(|(index, window)| (window == MARKER).then_some(index))
        .collect::<Vec<_>>();
    if positions.len() != 1 {
        return Err(error(DistributionErrorId::ObjectUnavailable));
    }
    let start = positions[0] + MARKER.len();
    let end = stdout[start..]
        .iter()
        .position(|byte| *byte == b'\n')
        .map_or(stdout.len(), |offset| start + offset);
    let value = &stdout[start..end];
    if value.is_empty() {
        return Err(error(DistributionErrorId::ProvenanceMismatch));
    }
    Ok(value)
}

fn read_output<T: Read>(stream: Option<T>) -> Result<Vec<u8>, DistributionError> {
    let mut bytes = Vec::new();
    stream
        .ok_or_else(|| error(DistributionErrorId::EffectFailed))?
        .take(OUTPUT_LIMIT as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| error(DistributionErrorId::EffectFailed))?;
    if bytes.len() > OUTPUT_LIMIT {
        return Err(error(DistributionErrorId::ObjectTooLarge));
    }
    Ok(bytes)
}

#[cfg(unix)]
fn executable_digest(path: &Path) -> Result<String, DistributionError> {
    use std::ffi::CString;
    use std::os::fd::FromRawFd;
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::MetadataExt;
    let name = CString::new(path.as_os_str().as_bytes())
        .map_err(|_| error(DistributionErrorId::InvalidPath))?;
    let descriptor = unsafe {
        libc::open(
            name.as_ptr(),
            libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
        )
    };
    if descriptor < 0 {
        return Err(error(DistributionErrorId::UnsafeObject));
    }
    let file = unsafe { std::fs::File::from_raw_fd(descriptor) };
    let before = file
        .metadata()
        .map_err(|_| error(DistributionErrorId::UnsafeObject))?;
    if !before.is_file() || before.nlink() != 1 || before.len() > EXECUTABLE_LIMIT as u64 {
        return Err(error(DistributionErrorId::UnsafeObject));
    }
    let mut bytes = Vec::with_capacity(before.len() as usize);
    file.take(EXECUTABLE_LIMIT as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| error(DistributionErrorId::ObjectUnavailable))?;
    let after =
        std::fs::symlink_metadata(path).map_err(|_| error(DistributionErrorId::ObjectChanged))?;
    if bytes.len() > EXECUTABLE_LIMIT
        || (before.dev(), before.ino(), before.len()) != (after.dev(), after.ino(), after.len())
    {
        return Err(error(DistributionErrorId::ObjectChanged));
    }
    Ok(sha256(&bytes))
}

#[cfg(not(unix))]
fn executable_digest(path: &Path) -> Result<String, DistributionError> {
    let metadata = std::fs::symlink_metadata(path)
        .map_err(|_| error(DistributionErrorId::ObjectUnavailable))?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() > EXECUTABLE_LIMIT as u64
    {
        return Err(error(DistributionErrorId::UnsafeObject));
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    std::fs::File::open(path)
        .and_then(|file| {
            file.take(EXECUTABLE_LIMIT as u64 + 1)
                .read_to_end(&mut bytes)
        })
        .map_err(|_| error(DistributionErrorId::ObjectUnavailable))?;
    if bytes.len() > EXECUTABLE_LIMIT {
        return Err(error(DistributionErrorId::ObjectTooLarge));
    }
    Ok(sha256(&bytes))
}
