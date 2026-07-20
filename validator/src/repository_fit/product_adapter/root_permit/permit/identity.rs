use super::*;

pub(crate) fn permit_id(
    authority_id: &str,
    binding: &PermitBinding,
    issued_tick: u64,
    expires_tick: u64,
    nonce_sha256: &str,
) -> Result<String, FitAdapterError> {
    serde_json::to_vec(&(
        PERMIT_DOMAIN,
        authority_id,
        binding,
        issued_tick,
        expires_tick,
        nonce_sha256,
    ))
    .map(|bytes| digest(&bytes))
    .map_err(|_| adapter_error(AdapterErrorId::ProjectionFailed))
}

pub(crate) fn capture_target(
    root: &Path,
    request: &OpaqueFitApplyRequest,
) -> Result<TargetSnapshot, FitAdapterError> {
    capture_target_descriptor_chain(root, request).map(|capture| capture.snapshot)
}

pub(crate) fn capture_target_descriptor_chain(
    root: &Path,
    request: &OpaqueFitApplyRequest,
) -> Result<TargetCapture, FitAdapterError> {
    let paths = request.target_paths();
    capture_target_descriptor_chain_for_paths(root, &paths)
}

pub(crate) fn capture_target_descriptor_chain_for_paths(
    root: &Path,
    target_paths: &[CanonicalPath],
) -> Result<TargetCapture, FitAdapterError> {
    #[cfg(not(target_vendor = "apple"))]
    {
        let _ = (root, target_paths);
        return Err(adapter_error(AdapterErrorId::UnsupportedHost));
    }
    #[cfg(target_vendor = "apple")]
    {
        let first = collect_target_descriptor_chain_for_paths(root, target_paths)?;
        test_target_capture_point(TargetCapturePhase::BeforeFinalChainRecheck, "");
        let final_recheck = collect_target_descriptor_chain_for_paths(root, target_paths)?;
        if first.snapshot != final_recheck.snapshot {
            return Err(adapter_error(AdapterErrorId::TargetUnavailable));
        }
        Ok(final_recheck)
    }
}

/// A complete descriptor-relative target collect. The public capture accepts
/// only two equal complete collects. Because each target object's Darwin ctime
/// is part of its row, the per-object stability intervals overlap at the
/// boundary between the collects and establish one common stable instant.
#[cfg(target_vendor = "apple")]
pub(crate) fn collect_target_descriptor_chain_for_paths(
    root: &Path,
    target_paths: &[CanonicalPath],
) -> Result<TargetCapture, FitAdapterError> {
    let root_path_metadata =
        fs::symlink_metadata(root).map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?;
    if root_path_metadata.file_type().is_symlink() || !root_path_metadata.is_dir() {
        return Err(adapter_error(AdapterErrorId::TargetUnavailable));
    }
    let mut options = OpenOptions::new();
    options
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK);
    let root_file = options
        .open(root)
        .map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?;
    let root_opened = root_file
        .metadata()
        .map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?;
    let root_object = metadata_object(&root_opened, None)?;
    if root_object.kind != "directory" || metadata_object(&root_path_metadata, None)? != root_object
    {
        return Err(adapter_error(AdapterErrorId::TargetUnavailable));
    }

    let mut rows = BTreeMap::from([(String::new(), TargetState::Present(root_object.clone()))]);
    let mut paths = Vec::with_capacity(target_paths.len());
    for target_path in target_paths {
        let components = target_path.components().collect::<Vec<_>>();
        let mut parent = root_file
            .try_clone()
            .map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?;
        let mut relative = String::new();
        let mut attachments = Vec::with_capacity(components.len());
        let mut missing = None;
        for (index, component) in components.iter().enumerate() {
            if !relative.is_empty() {
                relative.push('/');
            }
            relative.push_str(component);
            if missing.is_some() {
                insert_target_state(&mut rows, &relative, TargetState::Missing)?;
                continue;
            }
            match exact_entry_at(&parent, component)? {
                ExactEntry::Alias => {
                    return Err(adapter_error(AdapterErrorId::TargetUnavailable));
                }
                ExactEntry::Absent => {
                    if named_object_at(&parent, component, None)?.is_some() {
                        return Err(adapter_error(AdapterErrorId::TargetUnavailable));
                    }
                    test_target_capture_point(
                        TargetCapturePhase::AfterMissingBeforeRecheck,
                        &relative,
                    );
                    insert_target_state(&mut rows, &relative, TargetState::Missing)?;
                    missing = Some(HeldMissingAttachment {
                        path: relative.clone(),
                        parent: parent
                            .try_clone()
                            .map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?,
                        name: (*component).to_owned(),
                    });
                }
                ExactEntry::Exact => {
                    let leaf = index + 1 == components.len();
                    let attachment =
                        capture_target_attachment(&parent, component, &relative, !leaf)?;
                    if !leaf && attachment.object.kind != "directory" {
                        return Err(adapter_error(AdapterErrorId::TargetUnavailable));
                    }
                    insert_target_state(
                        &mut rows,
                        &relative,
                        TargetState::Present(attachment.object.clone()),
                    )?;
                    if leaf {
                        test_target_capture_point(
                            TargetCapturePhase::AfterLeafRevalidated,
                            &relative,
                        );
                    } else {
                        test_target_capture_point(
                            TargetCapturePhase::AfterParentHeldBeforeDescend,
                            &relative,
                        );
                        parent = attachment
                            .opened
                            .as_ref()
                            .ok_or_else(|| adapter_error(AdapterErrorId::TargetUnavailable))?
                            .try_clone()
                            .map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?;
                    }
                    attachments.push(attachment);
                }
            }
        }
        paths.push(HeldTargetPath {
            attachments,
            missing,
        });
    }

    let rows = rows
        .into_iter()
        .map(|(path, state)| TargetRow { path, state })
        .collect::<Vec<_>>();
    let sha256 = digest(
        &serde_json::to_vec(&rows).map_err(|_| adapter_error(AdapterErrorId::ProjectionFailed))?,
    );
    let chain = TargetDescriptorChain {
        root_path: root.to_path_buf(),
        root: root_file,
        root_object,
        paths,
    };
    Ok(TargetCapture {
        snapshot: TargetSnapshot { sha256, rows },
        chain,
    })
}

pub(crate) fn insert_target_state(
    rows: &mut BTreeMap<String, TargetState>,
    path: &str,
    state: TargetState,
) -> Result<(), FitAdapterError> {
    if rows.get(path).is_some_and(|prior| prior != &state) {
        return Err(adapter_error(AdapterErrorId::TargetUnavailable));
    }
    rows.insert(path.to_owned(), state);
    Ok(())
}
