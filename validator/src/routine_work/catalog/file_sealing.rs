use super::*;

#[cfg(unix)]
pub(crate) fn capture_root(root: &Path) -> CatalogResult<DirectoryIdentity> {
    if !root.is_absolute() || root.to_str().is_none() {
        return Err(error("catalog-worktree-root-invalid"));
    }
    let metadata =
        fs::symlink_metadata(root).map_err(|_| error("catalog-worktree-root-missing"))?;
    if !metadata.file_type().is_dir() || metadata.file_type().is_symlink() {
        return Err(error("catalog-worktree-root-unsafe"));
    }
    Ok(directory_identity(".", &metadata))
}

#[cfg(not(unix))]
pub(crate) fn capture_root(_root: &Path) -> CatalogResult<DirectoryIdentity> {
    Err(error("catalog-platform-unsupported"))
}

#[cfg(unix)]
pub(crate) fn capture_regular_file(
    root: &Path,
    root_identity: &DirectoryIdentity,
    absolute: &Path,
    relative: Option<&CatalogPath>,
    maximum: u64,
    require_root_device: bool,
) -> CatalogResult<(FileIdentity, Vec<u8>)> {
    let ancestors = if let Some(relative) = relative {
        capture_ancestors(root, root_identity, relative)?
    } else {
        capture_absolute_ancestors(absolute)?
    };
    let before = fs::symlink_metadata(absolute).map_err(|_| error("catalog-file-missing"))?;
    validate_regular_metadata(&before, root_identity.device, maximum, require_root_device)?;
    let mut options = OpenOptions::new();
    options
        .read(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK);
    let mut file = options
        .open(absolute)
        .map_err(|_| error("catalog-file-open-failed"))?;
    let opened = file
        .metadata()
        .map_err(|_| error("catalog-file-metadata-failed"))?;
    validate_regular_metadata(&opened, root_identity.device, maximum, require_root_device)?;
    if file_metadata_tuple(&before) != file_metadata_tuple(&opened) {
        return Err(error("catalog-file-open-race"));
    }
    let mut bytes = Vec::with_capacity((opened.len().min(maximum)) as usize);
    file.by_ref()
        .take(maximum.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|_| error("catalog-file-read-failed"))?;
    if bytes.len() as u64 > maximum || bytes.len() as u64 != opened.len() {
        return Err(error("catalog-file-size-invalid"));
    }
    let after = file
        .metadata()
        .map_err(|_| error("catalog-file-metadata-failed"))?;
    if file_metadata_tuple(&opened) != file_metadata_tuple(&after) {
        return Err(error("catalog-file-read-race"));
    }
    let current_root = capture_root(root)?;
    let current_ancestors = if let Some(relative) = relative {
        capture_ancestors(root, &current_root, relative)?
    } else {
        capture_absolute_ancestors(absolute)?
    };
    let ancestors_match = if relative.is_some() {
        current_ancestors == ancestors
    } else {
        same_runner_ancestors(&current_ancestors, &ancestors)
    };
    if &current_root != root_identity || !ancestors_match {
        return Err(error("catalog-file-ancestor-race"));
    }
    Ok((
        FileIdentity {
            device: opened.dev(),
            inode: opened.ino(),
            unix_mode: opened.mode(),
            owner_user_id: opened.uid(),
            owner_group_id: opened.gid(),
            link_count: opened.nlink(),
            byte_length: opened.len(),
            modified_seconds: opened.mtime(),
            modified_nanos: opened.mtime_nsec(),
            changed_seconds: opened.ctime(),
            changed_nanos: opened.ctime_nsec(),
            sha256: sha256(&bytes),
            ancestors,
        },
        bytes,
    ))
}

#[cfg(not(unix))]
pub(crate) fn capture_regular_file(
    _root: &Path,
    _root_identity: &DirectoryIdentity,
    _absolute: &Path,
    _relative: Option<&CatalogPath>,
    _maximum: u64,
    _require_root_device: bool,
) -> CatalogResult<(FileIdentity, Vec<u8>)> {
    Err(error("catalog-platform-unsupported"))
}

#[cfg(unix)]
pub(crate) fn validate_regular_metadata(
    metadata: &fs::Metadata,
    expected_device: u64,
    maximum: u64,
    require_expected_device: bool,
) -> CatalogResult<()> {
    if !metadata.file_type().is_file()
        || metadata.file_type().is_symlink()
        || metadata.nlink() != 1
        || (require_expected_device && metadata.dev() != expected_device)
    {
        return Err(error("catalog-file-object-unsafe"));
    }
    if metadata.len() > maximum {
        return Err(error("catalog-file-size-invalid"));
    }
    Ok(())
}

#[cfg(unix)]
pub(crate) fn capture_program(observation: &RunnerObservation) -> CatalogResult<SealedFile> {
    let root_path = Path::new("/");
    let root_identity = capture_root(root_path)?;
    let (identity, bytes) = capture_regular_file(
        root_path,
        &root_identity,
        &observation.executable_path,
        None,
        MAX_READ_SOURCE_BYTES,
        false,
    )?;
    if identity.sha256 != observation.program_sha256
        || identity.byte_length != observation.program_byte_length
        || identity.unix_mode != observation.program_unix_mode
    {
        return Err(error("catalog-runner-program-identity-stale"));
    }
    if identity.unix_mode & 0o111 == 0 || identity.unix_mode & 0o022 != 0 {
        return Err(error("catalog-runner-program-mutable"));
    }
    Ok(SealedFile {
        root: root_path.to_path_buf(),
        relative: None,
        absolute: observation.executable_path.clone(),
        root_identity,
        identity,
        bytes,
    })
}

#[cfg(not(unix))]
pub(crate) fn capture_program(_observation: &RunnerObservation) -> CatalogResult<SealedFile> {
    Err(error("catalog-platform-unsupported"))
}
