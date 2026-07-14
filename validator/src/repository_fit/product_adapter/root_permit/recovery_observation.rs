use super::*;

pub(crate) fn observe_recovery_target_contract(
    root: &Path,
    leaf_paths: &[CanonicalPath],
    contract: &ManagedAncestorContract,
) -> Result<RecoveryTargetContractObservation, FitAdapterError> {
    let leaf_path_strings = leaf_paths
        .iter()
        .map(|path| path.as_str().to_owned())
        .collect::<Vec<_>>();
    if !contract.valid_for_leaf_paths(&leaf_path_strings) {
        return Err(adapter_error(AdapterErrorId::ApplyPermitInvalid));
    }
    let mut capture = capture_target_descriptor_chain_for_paths(root, leaf_paths)?;
    capture.chain.revalidate()?;
    let retained_root_path = retained_descriptor_path(&capture.chain.root)?;
    let (ancestor_preimage, ancestor_postimage) =
        classify_managed_ancestor_contract(&capture.snapshot, contract)?;
    let rows = capture
        .snapshot
        .rows
        .iter()
        .map(|row| (row.path.as_str(), &row.state))
        .collect::<BTreeMap<_, _>>();
    let root_object = present_target_row(&rows, "")
        .ok_or_else(|| adapter_error(AdapterErrorId::ApplyOutcomeInvalid))?;
    let leaves = leaf_paths
        .iter()
        .map(|path| {
            let state = rows
                .get(path.as_str())
                .copied()
                .ok_or_else(|| adapter_error(AdapterErrorId::ApplyOutcomeInvalid))?;
            Ok(match state {
                TargetState::Missing => RecoveryLeafObservation {
                    path: path.as_str().to_owned(),
                    payload_sha256: None,
                    mode: None,
                    valid_managed_leaf: false,
                },
                TargetState::Present(object) => RecoveryLeafObservation {
                    path: path.as_str().to_owned(),
                    payload_sha256: object.payload_sha256.clone(),
                    mode: Some(object.mode & 0o7777),
                    valid_managed_leaf: object.kind == "regular"
                        && object.device == root_object.device
                        && object.links == 1
                        && object.uid == root_object.uid
                        && object.gid == root_object.gid,
                },
            })
        })
        .collect::<Result<Vec<_>, FitAdapterError>>()?;
    capture.chain.revalidate()?;
    if retained_descriptor_path(&capture.chain.root)? != retained_root_path {
        return Err(adapter_error(AdapterErrorId::TargetUnavailable));
    }
    let root_binding = root_binding_from_canonical_path(
        &retained_root_path,
        root_object.device,
        root_object.inode,
    );
    Ok(RecoveryTargetContractObservation {
        root_binding,
        ancestor_preimage,
        ancestor_postimage,
        leaves,
    })
}

#[cfg(target_vendor = "apple")]
pub(crate) fn retained_descriptor_path(root: &File) -> Result<PathBuf, FitAdapterError> {
    let mut buffer = [0 as libc::c_char; libc::PATH_MAX as usize];
    if unsafe { libc::fcntl(root.as_raw_fd(), libc::F_GETPATH, buffer.as_mut_ptr()) } < 0 {
        return Err(adapter_error(AdapterErrorId::TargetUnavailable));
    }
    let bytes = unsafe { CStr::from_ptr(buffer.as_ptr()) }.to_bytes();
    if bytes.first() != Some(&b'/') {
        return Err(adapter_error(AdapterErrorId::TargetUnavailable));
    }
    Ok(PathBuf::from(OsStr::from_bytes(bytes)))
}

#[cfg(not(target_vendor = "apple"))]
pub(crate) fn retained_descriptor_path(root: &File) -> Result<PathBuf, FitAdapterError> {
    let _ = root;
    Err(adapter_error(AdapterErrorId::UnsupportedHost))
}

pub(crate) fn managed_ancestor_paths(request: &OpaqueFitApplyRequest) -> BTreeSet<String> {
    let target_paths = request
        .desired
        .files
        .iter()
        .map(|file| file.path.clone())
        .collect::<Vec<_>>();
    managed_ancestor_paths_for_targets(&target_paths)
}

pub(crate) fn managed_ancestor_paths_for_targets(
    target_paths: &[CanonicalPath],
) -> BTreeSet<String> {
    let mut paths = BTreeSet::from([String::new()]);
    for path in target_paths {
        let components = path.components().collect::<Vec<_>>();
        let mut relative = String::new();
        for component in &components[..components.len() - 1] {
            if !relative.is_empty() {
                relative.push('/');
            }
            relative.push_str(component);
            paths.insert(relative.clone());
        }
    }
    paths
}

pub(crate) fn present_target_row<'a>(
    rows: &BTreeMap<&str, &'a TargetState>,
    path: &str,
) -> Option<&'a ObjectRow> {
    match rows.get(path) {
        Some(TargetState::Present(object)) => Some(object),
        Some(TargetState::Missing) | None => None,
    }
}

pub(crate) fn target_rollback_equivalent(
    current: &TargetSnapshot,
    expected: &TargetSnapshot,
    request: &OpaqueFitApplyRequest,
) -> bool {
    if current.rows.len() != expected.rows.len() {
        return false;
    }
    let leaves = request
        .desired
        .files
        .iter()
        .map(|file| file.path.as_str())
        .collect::<BTreeSet<_>>();
    current
        .rows
        .iter()
        .zip(&expected.rows)
        .all(|(left, right)| {
            if left.path != right.path {
                return false;
            }
            let leaf = leaves.contains(left.path.as_str());
            match (&left.state, &right.state) {
                (TargetState::Missing, TargetState::Missing) => true,
                (TargetState::Present(left_object), TargetState::Present(right_object)) => {
                    left_object.rollback_equivalent(right_object, leaf)
                }
                _ => false,
            }
        })
}

pub(crate) fn capture_protected(
    root: &Path,
    request: &OpaqueFitApplyRequest,
    boundary: ProtectedCaptureBoundary,
) -> Result<ProtectedSnapshot, FitAdapterError> {
    let first = collect_protected(root, request, boundary, true)?;
    test_protected_capture_point(boundary, ProtectedCapturePhase::BeforeFinalRecheck, b"");
    let final_recheck = collect_protected(root, request, boundary, false)?;
    if first != final_recheck {
        return Err(adapter_error(AdapterErrorId::TargetUnavailable));
    }
    Ok(final_recheck)
}
