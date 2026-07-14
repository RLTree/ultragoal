use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthorizedPath {
    pub(crate) canonical_path: PathBuf,
    pub(crate) anchor_path: PathBuf,
    pub(crate) anchor_identity: Option<(u64, u64)>,
    pub(crate) existed: bool,
    pub(crate) worktree_root: PathBuf,
    pub(crate) worktree_identity: Option<(u64, u64)>,
    pub(crate) write_scopes: Vec<PathBuf>,
    pub(crate) effect: EffectClass,
    pub(crate) live_context: LiveContext,
}

impl AuthorizedPath {
    pub(crate) fn existed(&self) -> bool {
        self.existed
    }

    pub(crate) fn revalidate(&self) -> Result<(), ContextError> {
        let worktree = self
            .worktree_root
            .canonicalize()
            .map_err(|error| io_error(&self.worktree_root, error))?;
        if worktree != self.worktree_root {
            return Err(ContextError::PathDenied(
                "worktree root identity changed".to_owned(),
            ));
        }
        #[cfg(unix)]
        if let Some((device, inode)) = self.worktree_identity {
            let metadata = fs::metadata(&worktree).map_err(|error| io_error(&worktree, error))?;
            if (metadata.dev(), metadata.ino()) != (device, inode) {
                return Err(ContextError::PathDenied(
                    "worktree root inode changed after authorization".to_owned(),
                ));
            }
        }
        let anchor = self
            .anchor_path
            .canonicalize()
            .map_err(|error| io_error(&self.anchor_path, error))?;
        if anchor != self.anchor_path {
            return Err(ContextError::PathDenied(format!(
                "path anchor changed from {} to {}",
                self.anchor_path.display(),
                anchor.display()
            )));
        }
        #[cfg(unix)]
        if let Some((device, inode)) = self.anchor_identity {
            let metadata = fs::metadata(&anchor).map_err(|error| io_error(&anchor, error))?;
            if (metadata.dev(), metadata.ino()) != (device, inode) {
                return Err(ContextError::PathDenied(format!(
                    "path anchor identity changed for {}",
                    anchor.display()
                )));
            }
        }
        if self.existed {
            let current = self
                .canonical_path
                .canonicalize()
                .map_err(|error| io_error(&self.canonical_path, error))?;
            if current != self.canonical_path {
                return Err(ContextError::PathDenied(format!(
                    "authorized path changed to {}",
                    current.display()
                )));
            }
            reject_hardlink(&current, self.effect)?;
        } else {
            self.revalidate_projected_components()?;
        }
        self.live_context.revalidate()?;
        Ok(())
    }

    pub(crate) fn revalidate_projected_components(&self) -> Result<(), ContextError> {
        let suffix = self
            .canonical_path
            .strip_prefix(&self.anchor_path)
            .map_err(|_| {
                ContextError::PathDenied("projected path no longer descends from anchor".to_owned())
            })?;
        let mut current = self.anchor_path.clone();
        let component_count = suffix.components().count();
        for (index, component) in suffix.components().enumerate() {
            current.push(component.as_os_str());
            let metadata = match fs::symlink_metadata(&current) {
                Ok(metadata) => metadata,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
                Err(error) => return Err(io_error(&current, error)),
            };
            if metadata.file_type().is_symlink() {
                return Err(ContextError::PathDenied(format!(
                    "new symlink component appeared in projected path: {}",
                    current.display()
                )));
            }
            let resolved = current
                .canonicalize()
                .map_err(|error| io_error(&current, error))?;
            self.check_confinement(&resolved)?;
            if index + 1 < component_count && !metadata.is_dir() {
                return Err(ContextError::PathDenied(format!(
                    "projected parent is not a directory: {}",
                    current.display()
                )));
            }
            if index + 1 == component_count {
                reject_hardlink(&resolved, self.effect)?;
            }
        }
        self.check_confinement(&self.canonical_path)
    }

    pub(crate) fn check_confinement(&self, path: &Path) -> Result<(), ContextError> {
        if !inside(path, &self.worktree_root) {
            return Err(ContextError::PathDenied(format!(
                "{} escapes worktree {}",
                path.display(),
                self.worktree_root.display()
            )));
        }
        if self.effect != EffectClass::Read
            && !self.write_scopes.iter().any(|scope| inside(path, scope))
        {
            return Err(ContextError::PathDenied(format!(
                "{} is outside declared write scopes",
                path.display()
            )));
        }
        Ok(())
    }
}

pub(crate) fn reject_lexical_escape(path: &Path) -> Result<(), ContextError> {
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(ContextError::PathDenied(format!(
            "parent traversal is not accepted: {}",
            path.display()
        )));
    }
    Ok(())
}

pub(crate) fn projected_canonical(path: &Path) -> Result<(PathBuf, PathBuf, bool), ContextError> {
    if path.exists() || fs::symlink_metadata(path).is_ok() {
        let canonical = path.canonicalize().map_err(|error| io_error(path, error))?;
        return Ok((canonical.clone(), canonical, true));
    }
    let mut missing = Vec::new();
    let mut existing = path;
    while !existing.exists() {
        let name = existing.file_name().ok_or_else(|| {
            ContextError::PathDenied(format!("no existing ancestor for {}", path.display()))
        })?;
        missing.push(name.to_owned());
        existing = existing.parent().ok_or_else(|| {
            ContextError::PathDenied(format!("no existing ancestor for {}", path.display()))
        })?;
    }
    let anchor = existing
        .canonicalize()
        .map_err(|error| io_error(existing, error))?;
    if !anchor.is_dir() {
        return Err(ContextError::PathDenied(format!(
            "nearest existing ancestor is not a directory: {}",
            anchor.display()
        )));
    }
    let mut projected = anchor.clone();
    for component in missing.iter().rev() {
        projected.push(component);
    }
    Ok((projected, anchor, false))
}

pub(crate) fn inside(path: &Path, root: &Path) -> bool {
    path == root || path.starts_with(root)
}
