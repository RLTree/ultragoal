use super::*;

#[cfg(target_vendor = "apple")]
pub(crate) fn append_protected_component(
    prefix: &[u8],
    name: &[u8],
) -> Result<Vec<u8>, FitAdapterError> {
    let required = prefix
        .len()
        .checked_add(usize::from(!prefix.is_empty()))
        .and_then(|length| length.checked_add(name.len()))
        .filter(|length| *length <= MAX_TARGET_NAME_BYTES)
        .ok_or_else(|| adapter_error(AdapterErrorId::TargetUnavailable))?;
    let mut relative = Vec::with_capacity(required);
    relative.extend_from_slice(prefix);
    if !prefix.is_empty() {
        relative.push(b'/');
    }
    relative.extend_from_slice(name);
    Ok(relative)
}

#[cfg(target_vendor = "apple")]
pub(crate) fn managed_protected_role(
    relative: &[u8],
    allowed: &BTreeSet<Vec<u8>>,
) -> Result<ManagedProtectedRole, FitAdapterError> {
    if allowed.contains(relative) {
        return Ok(ManagedProtectedRole::ExactLeaf);
    }
    if allowed
        .iter()
        .any(|target| protected_descendant(target, relative))
    {
        return Ok(ManagedProtectedRole::StrictAncestor);
    }
    let folded = ascii_fold(relative);
    if allowed.iter().any(|target| {
        let target = ascii_fold(target);
        target == folded || protected_descendant(&target, &folded)
    }) {
        return Err(adapter_error(AdapterErrorId::TargetUnavailable));
    }
    Ok(ManagedProtectedRole::Protected)
}

#[cfg(target_vendor = "apple")]
pub(crate) fn protected_descendant(candidate: &[u8], ancestor: &[u8]) -> bool {
    candidate.len() > ancestor.len()
        && candidate.starts_with(ancestor)
        && candidate.get(ancestor.len()) == Some(&b'/')
}

#[cfg(target_vendor = "apple")]
pub(crate) fn ascii_fold(value: &[u8]) -> Vec<u8> {
    value.iter().map(u8::to_ascii_lowercase).collect()
}

#[cfg(target_vendor = "apple")]
pub(crate) fn enumerate_protected_entries(
    directory: &File,
) -> Result<Vec<ProtectedEnumeratedEntry>, FitAdapterError> {
    let duplicated = unsafe { libc::dup(directory.as_raw_fd()) };
    if duplicated < 0 {
        return Err(adapter_error(AdapterErrorId::TargetUnavailable));
    }
    if unsafe { libc::fcntl(duplicated, libc::F_SETFD, libc::FD_CLOEXEC) } != 0 {
        unsafe {
            libc::close(duplicated);
        }
        return Err(adapter_error(AdapterErrorId::TargetUnavailable));
    }
    let stream = unsafe { libc::fdopendir(duplicated) };
    if stream.is_null() {
        unsafe {
            libc::close(duplicated);
        }
        return Err(adapter_error(AdapterErrorId::TargetUnavailable));
    }
    let stream = ProtectedDirectoryStream(stream);
    unsafe {
        libc::rewinddir(stream.0);
    }
    let mut entries = Vec::new();
    let mut name_bytes = 0_usize;
    loop {
        unsafe {
            *libc::__error() = 0;
        }
        let entry = unsafe { libc::readdir(stream.0) };
        if entry.is_null() {
            if last_errno() != 0 {
                return Err(adapter_error(AdapterErrorId::TargetUnavailable));
            }
            break;
        }
        let name = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
        if matches!(name, b"." | b"..") {
            continue;
        }
        name_bytes = name_bytes
            .checked_add(name.len())
            .filter(|bytes| *bytes <= MAX_TARGET_NAME_BYTES)
            .ok_or_else(|| adapter_error(AdapterErrorId::TargetUnavailable))?;
        if entries.len() >= MAX_FENCE_ENTRIES || unsafe { (*entry).d_ino } == 0 {
            return Err(adapter_error(AdapterErrorId::TargetUnavailable));
        }
        entries.push(ProtectedEnumeratedEntry {
            name: name.to_vec(),
            inode: unsafe { (*entry).d_ino as u64 },
        });
    }
    entries.sort_by(|left, right| left.name.cmp(&right.name));
    let mut folded = BTreeSet::new();
    for entry in &entries {
        if !folded.insert(ascii_fold(&entry.name)) {
            return Err(adapter_error(AdapterErrorId::TargetUnavailable));
        }
    }
    Ok(entries)
}

#[cfg(target_vendor = "apple")]
pub(crate) struct ProtectedDirectoryStream(*mut libc::DIR);

#[cfg(target_vendor = "apple")]
impl Drop for ProtectedDirectoryStream {
    fn drop(&mut self) {
        unsafe {
            libc::closedir(self.0);
        }
    }
}

#[cfg(target_vendor = "apple")]
pub(crate) fn open_target_at_bytes(
    parent: &File,
    name: &[u8],
    flags: i32,
) -> Result<File, FitAdapterError> {
    let name = CString::new(name).map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?;
    let descriptor = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) };
    if descriptor < 0 {
        return Err(adapter_error(AdapterErrorId::TargetUnavailable));
    }
    Ok(unsafe { File::from_raw_fd(descriptor) })
}

#[cfg(target_vendor = "apple")]
pub(crate) fn named_object_at_bytes(
    parent: &File,
    name: &[u8],
    payload_sha256: Option<String>,
) -> Result<Option<ObjectRow>, FitAdapterError> {
    let name = CString::new(name).map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?;
    let mut stat = MaybeUninit::<libc::stat>::zeroed();
    let result = unsafe {
        libc::fstatat(
            parent.as_raw_fd(),
            name.as_ptr(),
            stat.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if result != 0 {
        return if last_errno() == libc::ENOENT {
            Ok(None)
        } else {
            Err(adapter_error(AdapterErrorId::TargetUnavailable))
        };
    }
    let stat = unsafe { stat.assume_init() };
    Ok(Some(stat_object(&stat, payload_sha256)?))
}

#[cfg(target_vendor = "apple")]
pub(crate) fn named_versioned_object_at_bytes(
    parent: &File,
    name: &[u8],
    payload_sha256: Option<String>,
) -> Result<Option<VersionedProtectedObject>, FitAdapterError> {
    let name = CString::new(name).map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?;
    let mut stat = MaybeUninit::<libc::stat>::zeroed();
    let result = unsafe {
        libc::fstatat(
            parent.as_raw_fd(),
            name.as_ptr(),
            stat.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if result != 0 {
        return if last_errno() == libc::ENOENT {
            Ok(None)
        } else {
            Err(adapter_error(AdapterErrorId::TargetUnavailable))
        };
    }
    let stat = unsafe { stat.assume_init() };
    Ok(Some(VersionedProtectedObject {
        object: stat_object(&stat, payload_sha256)?,
        change_version: ProtectedChangeVersion {
            ctime_seconds: stat.st_ctime,
            ctime_nanoseconds: stat.st_ctime_nsec,
        },
    }))
}
