impl ConfinedRoot {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    pub(crate) fn open_workspace(
        context: &crate::context::LiveContext,
    ) -> Result<Self, DistributionError> {
        context
            .effect()
            .authorize(crate::context::EffectClass::WorkspaceWrite)
            .map_err(|_| error(DistributionErrorId::CapabilityMismatch))?;
        context
            .revalidate()
            .map_err(|_| error(DistributionErrorId::ObjectChanged))?;
        let canonical = context
            .worktree_root()
            .canonicalize()
            .map_err(|_| error(DistributionErrorId::ObjectUnavailable))?;
        if canonical != context.worktree_root()
            || context.effect().write_scopes.as_slice() != [canonical.to_string_lossy()]
        {
            return Err(error(DistributionErrorId::CapabilityMismatch));
        }
        let parent = canonical
            .parent()
            .ok_or_else(|| error(DistributionErrorId::InvalidPath))?
            .to_path_buf();
        let name = canonical
            .file_name()
            .and_then(|row| row.to_str())
            .ok_or_else(|| error(DistributionErrorId::InvalidPath))?
            .to_owned();
        let root = Self::open_bound(canonical, &parent, &name)?;
        if !context.matches_worktree_directory(
            root.authority.identity.device,
            root.authority.identity.inode,
        ) {
            return Err(error(DistributionErrorId::ObjectChanged));
        }
        context
            .revalidate()
            .map_err(|_| error(DistributionErrorId::ObjectChanged))?;
        Ok(root)
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    pub(crate) fn open_workspace(
        _context: &crate::context::LiveContext,
    ) -> Result<Self, DistributionError> {
        Err(error(DistributionErrorId::CapabilityMismatch))
    }
}
