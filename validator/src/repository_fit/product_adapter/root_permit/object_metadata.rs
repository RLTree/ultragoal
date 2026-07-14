use super::*;

pub(crate) fn metadata_object(
    metadata: &fs::Metadata,
    payload_sha256: Option<String>,
) -> Result<ObjectRow, FitAdapterError> {
    #[cfg(not(unix))]
    {
        let _ = (metadata, payload_sha256);
        return Err(adapter_error(AdapterErrorId::UnsupportedHost));
    }
    #[cfg(unix)]
    {
        let file_type = metadata.file_type();
        let kind = if file_type.is_symlink() {
            "symlink"
        } else if metadata.is_dir() {
            "directory"
        } else if metadata.is_file() {
            "regular"
        } else if file_type.is_fifo() {
            "fifo"
        } else if file_type.is_socket() {
            "socket"
        } else {
            "special"
        };
        Ok(ObjectRow {
            kind,
            device: metadata.dev(),
            inode: metadata.ino(),
            links: metadata.nlink(),
            uid: metadata.uid(),
            gid: metadata.gid(),
            mode: metadata.mode(),
            byte_length: metadata.len(),
            payload_sha256,
            change_version: ProtectedChangeVersion {
                ctime_seconds: metadata.ctime(),
                ctime_nanoseconds: metadata.ctime_nsec(),
            },
        })
    }
}

#[cfg(target_vendor = "apple")]
pub(crate) fn stable_descriptor_file_digest(file: &mut File) -> Result<String, FitAdapterError> {
    let before = file
        .metadata()
        .map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?;
    if !before.is_file() || before.len() > MAX_FENCE_FILE_BYTES {
        return Err(adapter_error(AdapterErrorId::TargetUnavailable));
    }
    file.seek(SeekFrom::Start(0))
        .map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?;
    let first = bounded_file_read(file, before.len())?;
    file.seek(SeekFrom::Start(0))
        .map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?;
    let second = bounded_file_read(file, before.len())?;
    let after = file
        .metadata()
        .map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?;
    if first != second
        || first.len() as u64 != before.len()
        || object_identity(&before) != object_identity(&after)
    {
        return Err(adapter_error(AdapterErrorId::TargetUnavailable));
    }
    Ok(digest(&first))
}

#[cfg(target_vendor = "apple")]
pub(crate) fn stable_readlink_at(parent: &File, name: &str) -> Result<String, FitAdapterError> {
    let first = readlink_at(parent, name)?;
    let second = readlink_at(parent, name)?;
    if first != second {
        return Err(adapter_error(AdapterErrorId::TargetUnavailable));
    }
    Ok(digest(&first))
}

#[cfg(target_vendor = "apple")]
pub(crate) fn readlink_at(parent: &File, name: &str) -> Result<Vec<u8>, FitAdapterError> {
    let name = CString::new(name).map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?;
    let mut bytes = vec![0_u8; 64 * 1024];
    let length = unsafe {
        libc::readlinkat(
            parent.as_raw_fd(),
            name.as_ptr(),
            bytes.as_mut_ptr().cast(),
            bytes.len(),
        )
    };
    if length < 0 || length as usize == bytes.len() {
        return Err(adapter_error(AdapterErrorId::TargetUnavailable));
    }
    bytes.truncate(length as usize);
    Ok(bytes)
}

#[cfg(target_vendor = "apple")]
pub(crate) fn exact_entry_at(parent: &File, name: &str) -> Result<ExactEntry, FitAdapterError> {
    let duplicated = parent
        .try_clone()
        .map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?
        .into_raw_fd();
    let directory = unsafe { libc::fdopendir(duplicated) };
    if directory.is_null() {
        unsafe {
            libc::close(duplicated);
        }
        return Err(adapter_error(AdapterErrorId::TargetUnavailable));
    }
    unsafe {
        libc::rewinddir(directory);
    }
    let expected = name.as_bytes();
    let mut exact = false;
    let mut alias = false;
    let mut entries = 0_usize;
    let mut name_bytes = 0_usize;
    loop {
        unsafe {
            *libc::__error() = 0;
        }
        let entry = unsafe { libc::readdir(directory) };
        if entry.is_null() {
            let failed = last_errno() != 0;
            unsafe {
                libc::closedir(directory);
            }
            if failed {
                return Err(adapter_error(AdapterErrorId::TargetUnavailable));
            }
            break;
        }
        let observed = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
        if matches!(observed, b"." | b"..") {
            continue;
        }
        entries += 1;
        name_bytes = name_bytes.saturating_add(observed.len());
        if entries > MAX_TARGET_ENTRY_SCAN || name_bytes > MAX_TARGET_NAME_BYTES {
            unsafe {
                libc::closedir(directory);
            }
            return Err(adapter_error(AdapterErrorId::TargetUnavailable));
        }
        if observed == expected {
            exact = true;
        } else if observed.eq_ignore_ascii_case(expected) {
            alias = true;
        }
    }
    Ok(if alias {
        ExactEntry::Alias
    } else if exact {
        ExactEntry::Exact
    } else {
        ExactEntry::Absent
    })
}

#[cfg(target_vendor = "apple")]
pub(crate) fn last_errno() -> i32 {
    unsafe { *libc::__error() }
}
