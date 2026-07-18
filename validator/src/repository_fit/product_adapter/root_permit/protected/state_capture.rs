use super::*;

/// A complete descriptor-relative protected-tree collect. `capture_protected`
/// accepts one only after a second complete collect returns the identical
/// versioned rows, so no row that changes after its first read can survive the
/// final recheck as the old protected state.
pub(crate) fn collect_protected(
    root: &Path,
    request: &OpaqueFitApplyRequest,
    boundary: ProtectedCaptureBoundary,
    run_capture_hooks: bool,
) -> Result<ProtectedSnapshot, FitAdapterError> {
    #[cfg(not(target_vendor = "apple"))]
    {
        let _ = (root, request, boundary, run_capture_hooks);
        return Err(adapter_error(AdapterErrorId::UnsupportedHost));
    }
    #[cfg(target_vendor = "apple")]
    {
        let path_metadata = fs::symlink_metadata(root)
            .map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?;
        if path_metadata.file_type().is_symlink() || !path_metadata.is_dir() {
            return Err(adapter_error(AdapterErrorId::TargetUnavailable));
        }
        let mut options = OpenOptions::new();
        options.read(true).custom_flags(
            libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
        );
        let mut root_descriptor = options
            .open(root)
            .map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?;
        let root_object = metadata_object(
            &root_descriptor
                .metadata()
                .map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?,
            None,
        )?;
        if root_object.kind != "directory" || metadata_object(&path_metadata, None)? != root_object
        {
            return Err(adapter_error(AdapterErrorId::TargetUnavailable));
        }
        let allowed = request
            .desired
            .files
            .iter()
            .map(|file| file.path.as_str().as_bytes().to_vec())
            .collect::<BTreeSet<_>>();
        let mut traversal = ProtectedDescriptorTraversal::new(
            root_object.device,
            allowed,
            boundary,
            run_capture_hooks,
            root_object.inode,
        );
        traversal.visit(&mut root_descriptor, &[], 0)?;
        let final_path_metadata = fs::symlink_metadata(root)
            .map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?;
        let final_root_object = metadata_object(
            &root_descriptor
                .metadata()
                .map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?,
            None,
        )?;
        if final_path_metadata.file_type().is_symlink()
            || metadata_object(&final_path_metadata, None)? != root_object
            || final_root_object != root_object
        {
            return Err(adapter_error(AdapterErrorId::TargetUnavailable));
        }
        let mut rows = traversal.into_rows();
        rows.sort_by(|left, right| left.path.cmp(&right.path));
        let sha256 = digest(
            &serde_json::to_vec(&rows)
                .map_err(|_| adapter_error(AdapterErrorId::ProjectionFailed))?,
        );
        Ok(ProtectedSnapshot { sha256, rows })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ProtectedEnumeratedEntry {
    pub(crate) name: Vec<u8>,
    pub(crate) inode: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ManagedProtectedRole {
    ExactLeaf,
    StrictAncestor,
    Protected,
}
