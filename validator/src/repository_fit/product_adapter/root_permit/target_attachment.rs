use super::*;

#[cfg(target_vendor = "apple")]
pub(crate) fn capture_target_attachment(
    parent: &File,
    name: &str,
    path: &str,
    require_directory: bool,
) -> Result<HeldTargetAttachment, FitAdapterError> {
    let before = named_object_at(parent, name, None)?
        .ok_or_else(|| adapter_error(AdapterErrorId::TargetUnavailable))?;
    test_target_capture_point(TargetCapturePhase::AfterNamedBeforeOpen, path);
    let (mut opened, payload_sha256) = match before.kind {
        "directory" => (
            Some(open_target_at(
                parent,
                name,
                libc::O_RDONLY
                    | libc::O_DIRECTORY
                    | libc::O_NOFOLLOW
                    | libc::O_CLOEXEC
                    | libc::O_NONBLOCK,
            )?),
            None,
        ),
        "regular" if !require_directory => {
            let mut file = open_target_at(
                parent,
                name,
                libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
            )?;
            test_target_capture_point(TargetCapturePhase::AfterLeafHeldBeforeRead, path);
            let digest = stable_descriptor_file_digest(&mut file)?;
            (Some(file), Some(digest))
        }
        "symlink" if !require_directory => (None, Some(stable_readlink_at(parent, name)?)),
        _ if !require_directory => (None, None),
        _ => return Err(adapter_error(AdapterErrorId::TargetUnavailable)),
    };
    let object = if let Some(file) = opened.as_mut() {
        metadata_object(
            &file
                .metadata()
                .map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?,
            payload_sha256,
        )?
    } else {
        named_object_at(parent, name, payload_sha256)?
            .ok_or_else(|| adapter_error(AdapterErrorId::TargetUnavailable))?
    };
    if exact_entry_at(parent, name)? != ExactEntry::Exact
        || named_object_at(parent, name, object.payload_sha256.clone())?.as_ref() != Some(&object)
        || !same_attachment_contract(&before, &object)
    {
        return Err(adapter_error(AdapterErrorId::TargetUnavailable));
    }
    Ok(HeldTargetAttachment {
        path: path.to_owned(),
        parent: parent
            .try_clone()
            .map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?,
        name: name.to_owned(),
        object,
        opened,
    })
}

impl TargetDescriptorChain {
    pub(crate) fn revalidate(&mut self) -> Result<(), FitAdapterError> {
        let path_root = fs::symlink_metadata(&self.root_path)
            .map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?;
        let opened_root = self
            .root
            .metadata()
            .map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?;
        if path_root.file_type().is_symlink()
            || metadata_object(&path_root, None)? != self.root_object
            || metadata_object(&opened_root, None)? != self.root_object
        {
            return Err(adapter_error(AdapterErrorId::TargetUnavailable));
        }
        for path in &mut self.paths {
            for attachment in &mut path.attachments {
                let payload = match (&mut attachment.opened, attachment.object.kind) {
                    (Some(opened), "regular") => Some(stable_descriptor_file_digest(opened)?),
                    (None, "symlink") => {
                        Some(stable_readlink_at(&attachment.parent, &attachment.name)?)
                    }
                    _ => None,
                };
                let opened_matches = match &attachment.opened {
                    Some(opened) => {
                        metadata_object(
                            &opened
                                .metadata()
                                .map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?,
                            payload.clone(),
                        )? == attachment.object
                    }
                    None => true,
                };
                if exact_entry_at(&attachment.parent, &attachment.name)? != ExactEntry::Exact
                    || !opened_matches
                    || named_object_at(&attachment.parent, &attachment.name, payload)?.as_ref()
                        != Some(&attachment.object)
                {
                    let _ = &attachment.path;
                    return Err(adapter_error(AdapterErrorId::TargetUnavailable));
                }
            }
            if let Some(missing) = &path.missing
                && (exact_entry_at(&missing.parent, &missing.name)? != ExactEntry::Absent
                    || named_object_at(&missing.parent, &missing.name, None)?.is_some())
            {
                let _ = &missing.path;
                return Err(adapter_error(AdapterErrorId::TargetUnavailable));
            }
        }
        Ok(())
    }
}

pub(crate) fn same_attachment_contract(left: &ObjectRow, right: &ObjectRow) -> bool {
    left.change_version == right.change_version
        && left.kind == right.kind
        && left.device == right.device
        && left.inode == right.inode
        && left.links == right.links
        && left.uid == right.uid
        && left.gid == right.gid
        && left.mode == right.mode
        && left.byte_length == right.byte_length
}

#[cfg(target_vendor = "apple")]
pub(crate) fn open_target_at(
    parent: &File,
    name: &str,
    flags: i32,
) -> Result<File, FitAdapterError> {
    let name = CString::new(name).map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?;
    // SAFETY: `parent` is an open directory descriptor, `name` is NUL-free,
    // and the caller supplies only flags valid for `openat`.
    let descriptor = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) };
    if descriptor < 0 {
        return Err(adapter_error(AdapterErrorId::TargetUnavailable));
    }
    // SAFETY: a non-negative `openat` result is a newly owned descriptor that
    // has not been wrapped or closed on any prior path.
    Ok(unsafe { File::from_raw_fd(descriptor) })
}

#[cfg(target_vendor = "apple")]
pub(crate) fn named_object_at(
    parent: &File,
    name: &str,
    payload_sha256: Option<String>,
) -> Result<Option<ObjectRow>, FitAdapterError> {
    let name = CString::new(name).map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?;
    let mut stat = MaybeUninit::<libc::stat>::zeroed();
    // SAFETY: `parent` is an open directory descriptor, `name` is NUL-free,
    // and `stat` points to valid writable storage for one libc stat value.
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
    // SAFETY: successful `fstatat` initialized the full stat value above.
    let stat = unsafe { stat.assume_init() };
    Ok(Some(stat_object(&stat, payload_sha256)?))
}

#[cfg(target_vendor = "apple")]
pub(crate) fn stat_object(
    stat: &libc::stat,
    payload_sha256: Option<String>,
) -> Result<ObjectRow, FitAdapterError> {
    let mode = stat.st_mode as u32;
    let kind = match mode & libc::S_IFMT as u32 {
        value if value == libc::S_IFDIR as u32 => "directory",
        value if value == libc::S_IFREG as u32 => "regular",
        value if value == libc::S_IFLNK as u32 => "symlink",
        value if value == libc::S_IFIFO as u32 => "fifo",
        value if value == libc::S_IFSOCK as u32 => "socket",
        _ => "special",
    };
    let byte_length = u64::try_from(stat.st_size)
        .map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?;
    Ok(ObjectRow {
        kind,
        device: stat.st_dev as u64,
        inode: stat.st_ino,
        links: stat.st_nlink as u64,
        uid: stat.st_uid,
        gid: stat.st_gid,
        mode,
        byte_length,
        payload_sha256,
        change_version: ProtectedChangeVersion {
            ctime_seconds: stat.st_ctime,
            ctime_nanoseconds: stat.st_ctime_nsec,
        },
    })
}
