use super::*;

pub(crate) fn require_desired_target<E: RepositoryFitPermitEffects>(
    root: &Path,
    request: &OpaqueFitApplyRequest,
    permit: &RepositoryFitApplyPermit,
    effects: &mut ScopedEffects<E>,
) -> Result<TargetSnapshot, FitAdapterError> {
    let authorized = effects.revalidate_authorized_target()?;
    let mut capture = capture_target_descriptor_chain(root, request)?;
    if capture.snapshot != authorized {
        return Err(adapter_error(AdapterErrorId::ApplyOutcomeInvalid));
    }
    capture.chain.revalidate()?;
    let snapshot = capture.snapshot;
    validate_managed_ancestors(&snapshot, &permit.target_prestate, request)?;
    let rows = snapshot
        .rows
        .iter()
        .map(|row| (row.path.as_str(), &row.state))
        .collect::<BTreeMap<_, _>>();
    let root_object = present_target_row(&rows, "")
        .ok_or_else(|| adapter_error(AdapterErrorId::ApplyOutcomeInvalid))?;
    for file in &request.desired.files {
        let object = present_target_row(&rows, file.path.as_str())
            .ok_or_else(|| adapter_error(AdapterErrorId::ApplyOutcomeInvalid))?;
        let expected_sha256 = file.sha256();
        if object.kind != "regular"
            || object.device != root_object.device
            || object.links != 1
            || object.uid != root_object.uid
            || object.gid != root_object.gid
            || object.mode & 0o7777 != request.unix_modes[file.path.as_str()]
            || object.payload_sha256.as_deref() != Some(expected_sha256.as_str())
        {
            return Err(adapter_error(AdapterErrorId::ApplyOutcomeInvalid));
        }
    }
    if let Some(local_state) = request.plan.local_state.as_ref() {
        let object = present_target_row(&rows, local_state.path.as_str())
            .ok_or_else(|| adapter_error(AdapterErrorId::ApplyOutcomeInvalid))?;
        if object.kind != "regular"
            || object.device != root_object.device
            || object.links != 1
            || object.uid != root_object.uid
            || object.gid != root_object.gid
            || object.mode & 0o7777 != local_state.desired_mode
            || object.payload_sha256.as_deref() != Some(local_state.desired_sha256().as_str())
        {
            return Err(adapter_error(AdapterErrorId::ApplyOutcomeInvalid));
        }
    }
    Ok(snapshot)
}

pub(crate) fn validate_managed_ancestors(
    current: &TargetSnapshot,
    expected: &TargetSnapshot,
    request: &OpaqueFitApplyRequest,
) -> Result<(), FitAdapterError> {
    let contract = managed_ancestor_contract(expected, request)?;
    let (_, postimage) = classify_managed_ancestor_contract(current, &contract)?;
    if postimage {
        Ok(())
    } else {
        Err(adapter_error(AdapterErrorId::ApplyOutcomeInvalid))
    }
}

pub(crate) fn managed_ancestor_contract(
    snapshot: &TargetSnapshot,
    request: &OpaqueFitApplyRequest,
) -> Result<ManagedAncestorContract, FitAdapterError> {
    let target_paths = request.target_paths();
    let paths = managed_ancestor_paths_for_targets(&target_paths);
    let snapshot_rows = snapshot
        .rows
        .iter()
        .map(|row| (row.path.as_str(), &row.state))
        .collect::<BTreeMap<_, _>>();
    let mut rows = Vec::with_capacity(paths.len());
    for path in paths {
        let expectation = match snapshot_rows.get(path.as_str()) {
            Some(TargetState::Present(object)) if object.kind == "directory" => {
                ManagedAncestorExpectation::Existing {
                    device: object.device,
                    inode: object.inode,
                    uid: object.uid,
                    gid: object.gid,
                    mode: object.mode,
                }
            }
            Some(TargetState::Missing) if !path.is_empty() => ManagedAncestorExpectation::Missing,
            _ => return Err(adapter_error(AdapterErrorId::ApplyOutcomeInvalid)),
        };
        rows.push(ManagedAncestorContractRow { path, expectation });
    }
    let contract = ManagedAncestorContract { rows };
    let leaf_paths = target_paths
        .iter()
        .map(|path| path.as_str().to_owned())
        .collect::<Vec<_>>();
    if contract.valid_for_leaf_paths(&leaf_paths) {
        Ok(contract)
    } else {
        Err(adapter_error(AdapterErrorId::ApplyOutcomeInvalid))
    }
}

pub(crate) fn classify_managed_ancestor_contract(
    current: &TargetSnapshot,
    contract: &ManagedAncestorContract,
) -> Result<(bool, bool), FitAdapterError> {
    let current_rows = current
        .rows
        .iter()
        .map(|row| (row.path.as_str(), &row.state))
        .collect::<BTreeMap<_, _>>();
    let current_root = present_target_row(&current_rows, "")
        .ok_or_else(|| adapter_error(AdapterErrorId::ApplyOutcomeInvalid))?;
    let mut preimage = true;
    let mut postimage = true;
    for row in &contract.rows {
        let current_state = current_rows
            .get(row.path.as_str())
            .copied()
            .ok_or_else(|| adapter_error(AdapterErrorId::ApplyOutcomeInvalid))?;
        match (&row.expectation, current_state) {
            (
                ManagedAncestorExpectation::Existing {
                    device,
                    inode,
                    uid,
                    gid,
                    mode,
                },
                TargetState::Present(object),
            ) => {
                let exact = object.kind == "directory"
                    && object.device == *device
                    && object.inode == *inode
                    && object.uid == *uid
                    && object.gid == *gid
                    && object.mode == *mode;
                preimage &= exact;
                postimage &= exact;
            }
            (ManagedAncestorExpectation::Missing, TargetState::Missing) => {
                postimage = false;
            }
            (ManagedAncestorExpectation::Missing, TargetState::Present(object)) => {
                preimage = false;
                postimage &= object.valid_created_managed_ancestor(current_root);
            }
            _ => {
                preimage = false;
                postimage = false;
            }
        }
    }
    Ok((preimage, postimage))
}
