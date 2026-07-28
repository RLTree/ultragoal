#[derive(Clone, Debug)]
pub(crate) struct ReadOnlyWorkspace {
    context: crate::context::LiveContext,
    root: ConfinedRoot,
}

impl ReadOnlyWorkspace {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    pub(crate) fn open(context: &crate::context::LiveContext) -> Result<Self, DistributionError> {
        if context.effect().selected != crate::context::EffectClass::Read
            || !context.effect().write_scopes.is_empty()
        {
            return Err(error(DistributionErrorId::CapabilityMismatch));
        }
        context
            .effect()
            .authorize(crate::context::EffectClass::Read)
            .map_err(|_| error(DistributionErrorId::CapabilityMismatch))?;
        context
            .revalidate()
            .map_err(|_| error(DistributionErrorId::ObjectChanged))?;
        let canonical = context
            .worktree_root()
            .canonicalize()
            .map_err(|_| error(DistributionErrorId::ObjectUnavailable))?;
        if canonical != context.worktree_root() {
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
        let root = ConfinedRoot::open_bound(canonical, &parent, &name)?;
        if !context.matches_worktree_directory(
            root.authority.identity.device,
            root.authority.identity.inode,
        ) {
            return Err(error(DistributionErrorId::ObjectChanged));
        }
        context
            .revalidate()
            .map_err(|_| error(DistributionErrorId::ObjectChanged))?;
        Ok(Self {
            context: context.clone(),
            root,
        })
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    pub(crate) fn open(_context: &crate::context::LiveContext) -> Result<Self, DistributionError> {
        Err(error(DistributionErrorId::CapabilityMismatch))
    }

    pub(crate) fn inspect_file(
        &self,
        relative: &str,
        maximum: usize,
    ) -> Result<Option<Vec<u8>>, DistributionError> {
        self.context
            .revalidate()
            .map_err(|_| error(DistributionErrorId::ObjectChanged))?;
        let bytes = super::file::ScopedFile::new(self.root.clone(), relative)?.inspect(maximum)?;
        self.context
            .revalidate()
            .map_err(|_| error(DistributionErrorId::ObjectChanged))?;
        Ok(bytes)
    }
}

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

#[cfg(all(test, unix))]
mod read_only_workspace_tests {
    use super::ReadOnlyWorkspace;
    use crate::distribution::DistributionErrorId;
    use std::fs;
    use std::path::Path;
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn read_context_opens_only_an_opaque_inspection_workspace() {
        let root = std::env::temp_dir().join(format!(
            "hul-read-workspace-{}-{}",
            std::process::id(),
            NEXT_ROOT.fetch_add(1, Ordering::Relaxed),
        ));
        fs::create_dir_all(root.join("target/ultragoal")).unwrap();
        run_git(&root, &["init", "-q"]);
        run_git(
            &root,
            &["config", "user.email", "distribution@example.invalid"],
        );
        run_git(&root, &["config", "user.name", "Distribution Test"]);
        fs::write(root.join("tracked.txt"), b"fixture\n").unwrap();
        run_git(&root, &["add", "tracked.txt"]);
        run_git(&root, &["commit", "-qm", "fixture"]);
        fs::write(root.join("target/ultragoal/package.hugpkg"), b"package").unwrap();

        let read =
            crate::context::LiveContext::build(crate::context::BuildRequest::new(&root)).unwrap();
        let workspace = ReadOnlyWorkspace::open(&read).unwrap();
        assert_eq!(
            workspace
                .inspect_file("target/ultragoal/package.hugpkg", 1024)
                .unwrap(),
            Some(b"package".to_vec())
        );
        let write = crate::context::LiveContext::build(
            crate::context::BuildRequest::new(&root).with_root_workspace_grant(&root),
        )
        .unwrap();
        assert_eq!(
            ReadOnlyWorkspace::open(&write).unwrap_err().id(),
            DistributionErrorId::CapabilityMismatch
        );
        fs::remove_dir_all(root).unwrap();
    }

    fn run_git(root: &Path, args: &[&str]) {
        assert!(
            Command::new("git")
                .args(args)
                .current_dir(root)
                .status()
                .unwrap()
                .success()
        );
    }
}
