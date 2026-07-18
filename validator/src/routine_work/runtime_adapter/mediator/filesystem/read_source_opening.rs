use super::*;

#[cfg(unix)]
pub(crate) fn open_read_source(
    root: &RootAnchor,
    relative: &RepoPath,
) -> Result<ReadSourceAnchor, RoutineError> {
    let components = relative.as_str().split('/').collect::<Vec<_>>();
    let (file_name, directory_components) = components
        .split_last()
        .ok_or_else(|| mediator_error("mediator-read-source-path-invalid"))?;
    let mut directory = root
        .file
        .try_clone()
        .map_err(|_| mediator_error("mediator-read-source-ancestor-open-failed"))?;
    let mut ancestors = Vec::with_capacity(directory_components.len() + 1);
    let mut ancestor_records = Vec::with_capacity(directory_components.len() + 1);
    let root_identity = ObjectIdentity::from(
        &directory
            .metadata()
            .map_err(|_| mediator_error("mediator-read-source-ancestor-metadata-failed"))?,
    );
    ancestors.push(ReadAncestorAnchor {
        file: directory
            .try_clone()
            .map_err(|_| mediator_error("mediator-read-source-ancestor-open-failed"))?,
        identity: root_identity,
    });
    ancestor_records.push(read_ancestor_record(String::new(), root_identity));
    let mut accumulated = String::new();
    for component in directory_components {
        let encoded = CString::new(component.as_bytes())
            .map_err(|_| mediator_error("mediator-read-source-path-invalid"))?;
        let child = open_read_component(&directory, &encoded, true)?;
        let identity = ObjectIdentity::from(
            &child
                .metadata()
                .map_err(|_| mediator_error("mediator-read-source-ancestor-metadata-failed"))?,
        );
        if identity.device != root.identity.device
            || identity.mode & u32::from(libc::S_IFMT) != u32::from(libc::S_IFDIR)
        {
            return Err(mediator_error("mediator-read-source-ancestor-unsafe"));
        }
        if !accumulated.is_empty() {
            accumulated.push('/');
        }
        accumulated.push_str(component);
        ancestors.push(ReadAncestorAnchor {
            file: child
                .try_clone()
                .map_err(|_| mediator_error("mediator-read-source-ancestor-open-failed"))?,
            identity,
        });
        ancestor_records.push(read_ancestor_record(accumulated.clone(), identity));
        directory = child;
    }
    let encoded = CString::new(file_name.as_bytes())
        .map_err(|_| mediator_error("mediator-read-source-path-invalid"))?;
    let mut file = open_read_component(&directory, &encoded, false)?;
    let before = ObjectIdentity::from(
        &file
            .metadata()
            .map_err(|_| mediator_error("mediator-read-source-metadata-failed"))?,
    );
    if before.device != root.identity.device
        || before.mode & u32::from(libc::S_IFMT) != u32::from(libc::S_IFREG)
        || before.links != 1
        || before.length > MAX_READ_SOURCE_BYTES
    {
        return Err(mediator_error("mediator-read-source-object-unsafe"));
    }
    let sha256 = digest_reader(&mut file, before.length)?;
    let after = ObjectIdentity::from(
        &file
            .metadata()
            .map_err(|_| mediator_error("mediator-read-source-metadata-failed"))?,
    );
    if before != after {
        return Err(RoutineError::new(
            RoutineErrorId::ConcurrentMutation,
            "mediator-read-source-mutated-during-capture",
            None,
        ));
    }
    let current = open_read_component(&directory, &encoded, false).map_err(|_| {
        RoutineError::new(
            RoutineErrorId::ConcurrentMutation,
            "mediator-read-source-replaced-during-capture",
            None,
        )
    })?;
    let current = ObjectIdentity::from(
        &current
            .metadata()
            .map_err(|_| mediator_error("mediator-read-source-metadata-failed"))?,
    );
    if current != before {
        return Err(RoutineError::new(
            RoutineErrorId::ConcurrentMutation,
            "mediator-read-source-replaced-during-capture",
            None,
        ));
    }
    file.seek(SeekFrom::Start(0))
        .map_err(|_| mediator_error("mediator-read-source-read-failed"))?;
    let record = RoutineReadSource {
        relative_path: relative.clone(),
        device: before.device,
        inode: before.inode,
        unix_mode: before.mode,
        owner_user_id: before.owner_user_id,
        owner_group_id: before.owner_group_id,
        link_count: before.links,
        byte_length: before.length,
        modified_seconds: before.modified_seconds,
        modified_nanos: before.modified_nanos,
        changed_seconds: before.changed_seconds,
        changed_nanos: before.changed_nanos,
        sha256,
        ancestors: ancestor_records,
    };
    Ok(ReadSourceAnchor {
        path: root.path.join(relative.as_str()),
        file,
        identity: before,
        ancestors,
        record,
    })
}

#[cfg(unix)]
pub(crate) fn open_read_component(
    directory: &File,
    name: &CStr,
    require_directory: bool,
) -> Result<File, RoutineError> {
    let directory_flag = if require_directory {
        libc::O_DIRECTORY
    } else {
        0
    };
    let descriptor = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_NONBLOCK | libc::O_NOFOLLOW | libc::O_CLOEXEC | directory_flag,
        )
    };
    if descriptor < 0 {
        return Err(mediator_error("mediator-read-source-object-unsafe"));
    }
    Ok(unsafe { File::from_raw_fd(descriptor) })
}

#[cfg(unix)]
pub(crate) fn read_ancestor_record(
    relative_directory: String,
    identity: ObjectIdentity,
) -> RoutineReadAncestor {
    RoutineReadAncestor {
        relative_directory,
        device: identity.device,
        inode: identity.inode,
        unix_mode: identity.mode,
        owner_user_id: identity.owner_user_id,
        owner_group_id: identity.owner_group_id,
    }
}

#[cfg(unix)]
pub(crate) fn ancestor_matches(record: &RoutineReadAncestor, identity: ObjectIdentity) -> bool {
    record.device == identity.device
        && record.inode == identity.inode
        && record.unix_mode == identity.mode
        && record.owner_user_id == identity.owner_user_id
        && record.owner_group_id == identity.owner_group_id
}

#[cfg(unix)]
pub(crate) fn read_source_record_matches(
    expected: &RoutineReadSource,
    current: &ReadSourceAnchor,
) -> bool {
    expected.relative_path == current.record.relative_path
        && source_matches(expected, current.identity, &current.record.sha256)
        && expected.ancestors.len() == current.record.ancestors.len()
        && expected
            .ancestors
            .iter()
            .zip(&current.record.ancestors)
            .all(|(expected, observed)| {
                expected.relative_directory == observed.relative_directory
                    && expected.device == observed.device
                    && expected.inode == observed.inode
                    && expected.unix_mode == observed.unix_mode
                    && expected.owner_user_id == observed.owner_user_id
                    && expected.owner_group_id == observed.owner_group_id
            })
}
