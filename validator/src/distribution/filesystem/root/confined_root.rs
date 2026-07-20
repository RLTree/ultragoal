#[derive(Clone, Debug)]
pub struct ConfinedRoot {
    canonical: PathBuf,
    root_id: String,
    #[cfg(unix)]
    authority: std::sync::Arc<RootAuthority>,
}

#[cfg(unix)]
#[derive(Debug)]
struct RootAuthority {
    parent: Directory,
    root: Directory,
    name: String,
    identity: DirectoryIdentity,
}

impl ConfinedRoot {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    pub fn open(path: &Path) -> Result<Self, DistributionError> {
        let temporary = std::env::var_os("CODEX_WORKTREE_TMP")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .canonicalize()
            .map_err(|_| error(DistributionErrorId::ObjectUnavailable))?;
        let canonical = path
            .canonicalize()
            .map_err(|_| error(DistributionErrorId::ObjectUnavailable))?;
        let metadata = std::fs::symlink_metadata(path)
            .map_err(|_| error(DistributionErrorId::ObjectUnavailable))?;
        let name = canonical
            .file_name()
            .and_then(|row| row.to_str())
            .ok_or_else(|| error(DistributionErrorId::InvalidPath))?
            .to_owned();
        if canonical.parent() != Some(temporary.as_path())
            || !name.starts_with("hul-distribution-")
            || !metadata.is_dir()
            || metadata.file_type().is_symlink()
        {
            return Err(error(DistributionErrorId::InvalidPath));
        }
        Self::open_bound(canonical, &temporary, &name)
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    fn open_bound(
        canonical: PathBuf,
        parent_path: &Path,
        name: &str,
    ) -> Result<Self, DistributionError> {
        let parent = Directory::open_path(parent_path)?;
        let root = parent.open_directory(name)?.retain_confined_root()?;
        let identity = root.identity();
        let root_id = sha256(
            format!(
                "{}\0{}\0{}",
                canonical.display(),
                identity.device,
                identity.inode
            )
            .as_bytes(),
        );
        let authority = RootAuthority {
            parent,
            root,
            name: name.to_owned(),
            identity,
        };
        authority.revalidate()?;
        Ok(Self {
            canonical,
            root_id,
            authority: std::sync::Arc::new(authority),
        })
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    pub fn open(_path: &Path) -> Result<Self, DistributionError> {
        Err(error(DistributionErrorId::CapabilityMismatch))
    }

    pub fn root_id(&self) -> &str {
        &self.root_id
    }

    /// Stable reporting identity only. All filesystem effects use the retained
    /// directory descriptors instead of reopening this pathname.
    pub fn canonical_path(&self) -> &Path {
        &self.canonical
    }

    #[cfg(unix)]
    pub(crate) fn revalidate(&self) -> Result<(), DistributionError> {
        self.authority.revalidate()
    }

    #[cfg(not(unix))]
    pub(crate) fn revalidate(&self) -> Result<(), DistributionError> {
        Err(error(DistributionErrorId::CapabilityMismatch))
    }

    #[cfg(unix)]
    pub(super) fn root_directory(&self) -> Result<Directory, DistributionError> {
        self.revalidate()?;
        self.authority.root.duplicate()
    }

    #[cfg(unix)]
    pub(super) fn parent(
        &self,
        relative: &str,
        create: bool,
    ) -> Result<(Directory, String), DistributionError> {
        validate_relative_path(relative)?;
        self.revalidate()?;
        let components = relative.split('/').collect::<Vec<_>>();
        let mut directory = self.authority.root.duplicate()?;
        for component in &components[..components.len() - 1] {
            directory = if create {
                directory.ensure_directory(component)?
            } else {
                directory.open_directory(component)?
            };
        }
        self.revalidate()?;
        Ok((directory, components[components.len() - 1].to_owned()))
    }

    #[cfg(unix)]
    pub(super) fn revalidate_parent(
        &self,
        relative: &str,
        expected: &Directory,
    ) -> Result<(), DistributionError> {
        let (current, _) = self.parent(relative, false)?;
        if current.identity() != expected.identity() {
            return Err(error(DistributionErrorId::ObjectChanged));
        }
        Ok(())
    }

    pub(super) fn validate_relative(&self, relative: &str) -> Result<(), DistributionError> {
        validate_relative_path(relative)?;
        self.revalidate()
    }
}

#[cfg(unix)]
impl RootAuthority {
    fn revalidate(&self) -> Result<(), DistributionError> {
        self.parent.verify_descriptor()?;
        self.root.verify_descriptor()?;
        let current = self.parent.stat(&self.name)?;
        if !current
            .is_some_and(|row| row.kind == EntryKind::Directory && row.identity == self.identity)
        {
            return Err(error(DistributionErrorId::ObjectChanged));
        }
        Ok(())
    }
}
