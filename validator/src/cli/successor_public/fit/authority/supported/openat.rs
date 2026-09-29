use super::*;

pub(crate) fn openat(
    directory: &File,
    name: &str,
    flags: libc::c_int,
    mode: libc::mode_t,
) -> Result<File, HostFailure> {
    let encoded = CString::new(name).map_err(|_| HostFailure::Invalid)?;
    // SAFETY: the directory descriptor is borrowed, the C string is NUL-terminated, and flags are caller-controlled constants.
    let descriptor = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            encoded.as_ptr(),
            flags,
            mode as libc::c_uint,
        )
    };
    if descriptor < 0 {
        return Err(HostFailure::Unavailable);
    }
    // SAFETY: a non-negative descriptor was returned exclusively to this call.
    Ok(unsafe { File::from_raw_fd(descriptor) })
}

pub(crate) fn mkdirat_owned(directory: &File, name: &str) -> Result<bool, HostFailure> {
    let encoded = CString::new(name).map_err(|_| HostFailure::Invalid)?;
    // SAFETY: the directory descriptor is borrowed and the C string is NUL-terminated.
    if unsafe { libc::mkdirat(directory.as_raw_fd(), encoded.as_ptr(), 0o700) } == 0 {
        sync_directory(directory)?;
        return Ok(true);
    }
    match std::io::Error::last_os_error().raw_os_error() {
        Some(libc::EEXIST) => Ok(false),
        _ => Err(HostFailure::Persistence),
    }
}

pub(crate) fn rename_exclusive(directory: &File, old: &str, new: &str) -> Result<(), HostFailure> {
    let old = CString::new(old).map_err(|_| HostFailure::Invalid)?;
    let new = CString::new(new).map_err(|_| HostFailure::Invalid)?;
    let flags = libc::RENAME_EXCL as libc::c_uint | RENAME_NOFOLLOW_ANY | RENAME_RESOLVE_BENEATH;
    // SAFETY: both descriptors are borrowed, both C strings are NUL-terminated, and flags are fixed constants.
    if unsafe {
        libc::renameatx_np(
            directory.as_raw_fd(),
            old.as_ptr(),
            directory.as_raw_fd(),
            new.as_ptr(),
            flags,
        )
    } != 0
    {
        return Err(HostFailure::Persistence);
    }
    Ok(())
}

pub(crate) fn unlink_at(directory: &File, name: &str) -> Result<(), HostFailure> {
    let encoded = CString::new(name).map_err(|_| HostFailure::Invalid)?;
    // SAFETY: the directory descriptor is borrowed and the C string is NUL-terminated.
    if unsafe { libc::unlinkat(directory.as_raw_fd(), encoded.as_ptr(), 0) } != 0 {
        return Err(HostFailure::Persistence);
    }
    Ok(())
}

pub(crate) fn sync_directory(directory: &File) -> Result<(), HostFailure> {
    // SAFETY: the directory descriptor is borrowed for the duration of this call.
    if unsafe { libc::fsync(directory.as_raw_fd()) } != 0 {
        return Err(HostFailure::Persistence);
    }
    Ok(())
}

pub(crate) fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

pub(crate) fn decode_hex(value: &str) -> Result<Vec<u8>, HostFailure> {
    if value.is_empty()
        || !value.len().is_multiple_of(2)
        || value.len() > MAX_PENDING_BYTES as usize * 2
    {
        return Err(HostFailure::Invalid);
    }
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let high = hex_digit(pair[0])?;
            let low = hex_digit(pair[1])?;
            Ok((high << 4) | low)
        })
        .collect()
}

pub(crate) fn hex_digit(value: u8) -> Result<u8, HostFailure> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        _ => Err(HostFailure::Invalid),
    }
}
