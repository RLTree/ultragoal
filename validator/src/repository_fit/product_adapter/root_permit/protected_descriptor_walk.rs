use super::*;

#[cfg(target_vendor = "apple")]
#[allow(clippy::too_many_arguments)]
pub(crate) fn visit_protected_descriptor(
    directory: &mut File,
    prefix: &[u8],
    root_device: u64,
    allowed: &BTreeSet<Vec<u8>>,
    boundary: ProtectedCaptureBoundary,
    run_capture_hooks: bool,
    depth: usize,
    budget: &mut FenceBudget,
    visited: &mut BTreeSet<(u64, u64)>,
    rows: &mut Vec<ProtectedRow>,
) -> Result<(), FitAdapterError> {
    if depth > MAX_FENCE_DEPTH {
        return Err(adapter_error(AdapterErrorId::TargetUnavailable));
    }
    let entries = enumerate_protected_entries(directory)?;
    budget.entries = budget
        .entries
        .checked_add(entries.len())
        .filter(|count| *count <= MAX_FENCE_ENTRIES)
        .ok_or_else(|| adapter_error(AdapterErrorId::TargetUnavailable))?;
    let observed_name_bytes = entries
        .iter()
        .try_fold(0_usize, |total, entry| total.checked_add(entry.name.len()));
    budget.name_bytes = budget
        .name_bytes
        .checked_add(
            observed_name_bytes.ok_or_else(|| adapter_error(AdapterErrorId::TargetUnavailable))?,
        )
        .filter(|count| *count <= MAX_TARGET_NAME_BYTES)
        .ok_or_else(|| adapter_error(AdapterErrorId::TargetUnavailable))?;

    for entry in &entries {
        let relative = append_protected_component(prefix, &entry.name)?;
        let role = managed_protected_role(&relative, allowed)?;
        if run_capture_hooks {
            test_protected_capture_point(
                boundary,
                ProtectedCapturePhase::AfterEnumerationBeforeChildOpen,
                &relative,
            );
        }
        let before = named_versioned_object_at_bytes(directory, &entry.name, None)?
            .ok_or_else(|| adapter_error(AdapterErrorId::TargetUnavailable))?;
        if before.object.inode != entry.inode || before.object.device != root_device {
            return Err(adapter_error(AdapterErrorId::TargetUnavailable));
        }
        match before.object.kind {
            "directory" => {
                if role == ManagedProtectedRole::ExactLeaf {
                    return Err(adapter_error(AdapterErrorId::TargetUnavailable));
                }
                let mut child = open_target_at_bytes(
                    directory,
                    &entry.name,
                    libc::O_RDONLY
                        | libc::O_DIRECTORY
                        | libc::O_NOFOLLOW
                        | libc::O_CLOEXEC
                        | libc::O_NONBLOCK,
                )?;
                let held_metadata = child
                    .metadata()
                    .map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?;
                let held = metadata_versioned_object(&held_metadata, None)?;
                if held != before || !visited.insert((held.object.device, held.object.inode)) {
                    return Err(adapter_error(AdapterErrorId::TargetUnavailable));
                }
                if role == ManagedProtectedRole::Protected {
                    rows.push(ProtectedRow {
                        path: relative.clone(),
                        object: held.object.clone(),
                        change_version: held.change_version,
                    });
                }
                if run_capture_hooks {
                    test_protected_capture_point(
                        boundary,
                        ProtectedCapturePhase::AfterDirectoryHeldBeforeDescend,
                        &relative,
                    );
                }
                visit_protected_descriptor(
                    &mut child,
                    &relative,
                    root_device,
                    allowed,
                    boundary,
                    run_capture_hooks,
                    depth + 1,
                    budget,
                    visited,
                    rows,
                )?;
                let held_after_metadata = child
                    .metadata()
                    .map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?;
                let held_after = metadata_versioned_object(&held_after_metadata, None)?;
                if held_after != held
                    || named_versioned_object_at_bytes(directory, &entry.name, None)?.as_ref()
                        != Some(&held)
                {
                    return Err(adapter_error(AdapterErrorId::TargetUnavailable));
                }
            }
            "regular" => {
                if role == ManagedProtectedRole::StrictAncestor || before.object.links != 1 {
                    return Err(adapter_error(AdapterErrorId::TargetUnavailable));
                }
                if role == ManagedProtectedRole::Protected {
                    budget.bytes = budget
                        .bytes
                        .checked_add(before.object.byte_length)
                        .filter(|bytes| *bytes <= MAX_FENCE_TOTAL_BYTES)
                        .ok_or_else(|| adapter_error(AdapterErrorId::TargetUnavailable))?;
                }
                let mut file = open_target_at_bytes(
                    directory,
                    &entry.name,
                    libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
                )?;
                let opened_metadata = file
                    .metadata()
                    .map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?;
                let opened = metadata_versioned_object(&opened_metadata, None)?;
                if opened != before {
                    return Err(adapter_error(AdapterErrorId::TargetUnavailable));
                }
                let payload_sha256 = stable_descriptor_file_digest(&mut file)?;
                let object_metadata = file
                    .metadata()
                    .map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?;
                let object =
                    metadata_versioned_object(&object_metadata, Some(payload_sha256.clone()))?;
                if !same_versioned_attachment_contract(&before, &object)
                    || named_versioned_object_at_bytes(
                        directory,
                        &entry.name,
                        Some(payload_sha256.clone()),
                    )?
                    .as_ref()
                        != Some(&object)
                {
                    return Err(adapter_error(AdapterErrorId::TargetUnavailable));
                }
                let mut named = open_target_at_bytes(
                    directory,
                    &entry.name,
                    libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
                )?;
                let named_before_metadata = named
                    .metadata()
                    .map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?;
                let named_before = metadata_versioned_object(&named_before_metadata, None)?;
                let named_after_digest = stable_descriptor_file_digest(&mut named)?;
                let named_after_metadata = named
                    .metadata()
                    .map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?;
                if !same_versioned_attachment_contract(&object, &named_before)
                    || named_after_digest != payload_sha256
                    || metadata_versioned_object(
                        &named_after_metadata,
                        Some(payload_sha256.clone()),
                    )? != object
                    || named_versioned_object_at_bytes(
                        directory,
                        &entry.name,
                        Some(payload_sha256),
                    )?
                    .as_ref()
                        != Some(&object)
                {
                    return Err(adapter_error(AdapterErrorId::TargetUnavailable));
                }
                if role == ManagedProtectedRole::Protected {
                    rows.push(ProtectedRow {
                        path: relative.clone(),
                        object: object.object,
                        change_version: object.change_version,
                    });
                    if run_capture_hooks {
                        test_protected_capture_point(
                            boundary,
                            ProtectedCapturePhase::AfterRegularRowRevalidated,
                            &relative,
                        );
                    }
                }
            }
            "symlink" | "fifo" | "socket" | "special" | _ => {
                return Err(adapter_error(AdapterErrorId::TargetUnavailable));
            }
        }
    }
    if enumerate_protected_entries(directory)? != entries {
        return Err(adapter_error(AdapterErrorId::TargetUnavailable));
    }
    Ok(())
}
