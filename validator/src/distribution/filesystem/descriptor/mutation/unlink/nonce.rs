static UNLINK_NONCE: AtomicU64 = AtomicU64::new(0);

pub(crate) fn rename_noreplace(
    old_parent: &Directory,
    old: &str,
    new_parent: &Directory,
    new: &str,
) -> Result<bool, DistributionError> {
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    return Err(error(DistributionErrorId::CapabilityMismatch));

    let old = component(old)?;
    let new = component(new)?;
    let old_text = old
        .to_str()
        .map_err(|_| error(DistributionErrorId::InvalidPath))?;
    let new_text = new
        .to_str()
        .map_err(|_| error(DistributionErrorId::InvalidPath))?;
    let detail = format!(
        "{}->{}",
        joined(old_parent.relative(), old_text),
        joined(new_parent.relative(), new_text),
    );
    hooks::before(EffectPoint::Rename, &detail);
    let old_directory = old_parent.mutation_descriptor()?;
    let new_directory = new_parent.mutation_descriptor()?;
    let result = super::mutation_syscall::rename_noreplace(
        old_directory.as_raw_fd(),
        old.as_ptr(),
        new_directory.as_raw_fd(),
        new.as_ptr(),
    );
    if result == 0 {
        return Ok(true);
    }
    match last_errno() {
        Some(libc::EEXIST) | Some(libc::ENOENT) => Ok(false),
        _ => Err(error(DistributionErrorId::EffectFailed)),
    }
}

pub(crate) fn rename_swap(
    left_parent: &Directory,
    left: &str,
    right_parent: &Directory,
    right: &str,
) -> Result<bool, DistributionError> {
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    return Err(error(DistributionErrorId::CapabilityMismatch));

    let left = component(left)?;
    let right = component(right)?;
    let left_text = left
        .to_str()
        .map_err(|_| error(DistributionErrorId::InvalidPath))?;
    let right_text = right
        .to_str()
        .map_err(|_| error(DistributionErrorId::InvalidPath))?;
    let detail = format!(
        "{}<->{}",
        joined(left_parent.relative(), left_text),
        joined(right_parent.relative(), right_text),
    );
    hooks::before(EffectPoint::Rename, &detail);
    let left_directory = left_parent.mutation_descriptor()?;
    let right_directory = right_parent.mutation_descriptor()?;
    let result = super::mutation_syscall::rename_swap(
        left_directory.as_raw_fd(),
        left.as_ptr(),
        right_directory.as_raw_fd(),
        right.as_ptr(),
    );
    if result == 0 {
        return Ok(true);
    }
    match last_errno() {
        Some(libc::ENOENT) => Ok(false),
        _ => Err(error(DistributionErrorId::EffectFailed)),
    }
}

pub(crate) fn unlink_file_identity(
    parent: &Directory,
    name: &str,
    expected: FileIdentity,
) -> Result<(), DistributionError> {
    unlink_entry_identity(
        parent,
        name,
        EntryMetadata {
            identity: DirectoryIdentity {
                device: expected.device,
                inode: expected.inode,
            },
            kind: EntryKind::Regular,
            links: 1,
            length: expected.length,
        },
    )
}

pub(crate) fn unlink_directory_identity(
    parent: &Directory,
    name: &str,
    expected: DirectoryIdentity,
) -> Result<(), DistributionError> {
    unlink_checked(
        parent,
        name,
        EntryMetadata {
            identity: expected,
            kind: EntryKind::Directory,
            links: 0,
            length: 0,
        },
        libc::AT_REMOVEDIR,
    )
}

pub(crate) fn unlink_entry_identity(
    parent: &Directory,
    name: &str,
    expected: EntryMetadata,
) -> Result<(), DistributionError> {
    unlink_checked(parent, name, expected, 0)
}
