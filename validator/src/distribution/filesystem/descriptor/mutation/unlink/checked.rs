fn unlink_checked(
    parent: &Directory,
    name: &str,
    expected: EntryMetadata,
    flags: libc::c_int,
) -> Result<(), DistributionError> {
    // The first no-replace rename is the removal linearization point for the
    // caller-visible name. It captures one directory entry without following
    // it. The second no-replace rename and identity check close the deterministic
    // post-quarantine substitution window before the final descriptor-relative
    // unlink. POSIX does not provide an inode-targeted unlink: the final
    // fstatat-to-unlinkat interval therefore assumes no concurrent actor that can
    // rename entries in this retained parent. The race tests exercise and
    // preserve substitutions at both explicit quarantine boundaries; they do
    // not claim safety against an arbitrary malicious same-UID scheduler in that
    // final interval.
    let original = component(name)?;
    let text = original
        .to_str()
        .map_err(|_| error(DistributionErrorId::InvalidPath))?;
    let nonce = UNLINK_NONCE.fetch_add(1, Ordering::Relaxed);
    let quarantine_name = format!(".hul-quarantine-{}-{nonce}", std::process::id());
    let disposal_name = format!(".hul-disposal-{}-{nonce}", std::process::id());
    let quarantine = component(&quarantine_name)?;
    let disposal = component(&disposal_name)?;
    hooks::before(EffectPoint::Quarantine, &joined(parent.relative(), text));
    if !rename_noreplace_raw(parent, &original, parent, &quarantine)? {
        return Err(error(DistributionErrorId::ObjectChanged));
    }
    let captured = parent.stat(&quarantine_name)?;
    if !captured.is_some_and(|row| entry_matches(row, expected)) {
        restore_name(parent, &quarantine, &original)?;
        return Err(error(DistributionErrorId::ObjectChanged));
    }
    hooks::before(
        EffectPoint::Unlink,
        &joined(parent.relative(), &quarantine_name),
    );
    if !rename_noreplace_raw(parent, &quarantine, parent, &disposal)? {
        if parent
            .stat(&quarantine_name)?
            .is_some_and(|row| entry_matches(row, expected))
        {
            restore_name(parent, &quarantine, &original)?;
        }
        return Err(error(DistributionErrorId::ObjectChanged));
    }
    let captured = parent.stat(&disposal_name)?;
    if !captured.is_some_and(|row| entry_matches(row, expected)) {
        restore_name(parent, &disposal, &quarantine)?;
        return Err(error(DistributionErrorId::ObjectChanged));
    }
    // SAFETY: the descriptor is reopened from the retained parent and `disposal` is NUL-terminated.
    if unsafe {
        let directory = parent.mutation_descriptor()?;
        libc::unlinkat(
            directory.as_raw_fd(),
            disposal.as_ptr(),
            hardened_unlink_flags(flags),
        )
    } != 0
    {
        if restore_name(parent, &disposal, &original).is_err() {
            return Err(error(DistributionErrorId::RollbackFailed));
        }
        return Err(error(DistributionErrorId::EffectFailed));
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn hardened_unlink_flags(flags: libc::c_int) -> libc::c_int {
    const AT_SYMLINK_NOFOLLOW_ANY: libc::c_int = 0x0800;
    const AT_RESOLVE_BENEATH: libc::c_int = 0x2000;
    const AT_UNIQUE: libc::c_int = 0x8000;
    let regular_only = if flags & libc::AT_REMOVEDIR == 0 {
        AT_UNIQUE
    } else {
        0
    };
    flags | AT_SYMLINK_NOFOLLOW_ANY | AT_RESOLVE_BENEATH | regular_only
}

#[cfg(not(target_os = "macos"))]
fn hardened_unlink_flags(flags: libc::c_int) -> libc::c_int {
    flags
}

fn rename_noreplace_raw(
    old_parent: &Directory,
    old: &std::ffi::CStr,
    new_parent: &Directory,
    new: &std::ffi::CStr,
) -> Result<bool, DistributionError> {
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

fn restore_name(
    parent: &Directory,
    from: &std::ffi::CStr,
    to: &std::ffi::CStr,
) -> Result<(), DistributionError> {
    if !rename_noreplace_raw(parent, from, parent, to)? {
        return Err(error(DistributionErrorId::RollbackFailed));
    }
    Ok(())
}

fn entry_matches(actual: EntryMetadata, expected: EntryMetadata) -> bool {
    actual.identity == expected.identity
        && actual.kind == expected.kind
        && (expected.kind == EntryKind::Directory
            || actual.links == expected.links && actual.length == expected.length)
}
