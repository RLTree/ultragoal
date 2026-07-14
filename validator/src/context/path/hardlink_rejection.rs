use super::*;

pub(crate) fn reject_hardlink(path: &Path, effect: EffectClass) -> Result<(), ContextError> {
    if !matches!(
        effect,
        EffectClass::WorkspaceWrite | EffectClass::Destructive
    ) {
        return Ok(());
    }
    let Ok(metadata) = fs::metadata(path) else {
        return Ok(());
    };
    #[cfg(unix)]
    if metadata.is_file() && metadata.nlink() > 1 {
        return Err(ContextError::PathDenied(format!(
            "write target has {} hard links: {}",
            metadata.nlink(),
            path.display()
        )));
    }
    Ok(())
}

impl LiveContext {
    pub(crate) fn authorize_path(
        &self,
        path: impl AsRef<Path>,
        effect: EffectClass,
    ) -> Result<AuthorizedPath, ContextError> {
        self.revalidate()?;
        self.effect().authorize(effect)?;
        if effect == EffectClass::ExternalWrite {
            return Err(ContextError::PathDenied(
                "external effects cannot be authorized as workspace paths".to_owned(),
            ));
        }
        let worktree = self.worktree_root();
        let requested = path.as_ref();
        reject_lexical_escape(requested)?;
        let joined = if requested.is_absolute() {
            requested.to_path_buf()
        } else {
            worktree.join(requested)
        };
        let (canonical, anchor, existed) = projected_canonical(&joined)?;
        if effect == EffectClass::Read && !existed {
            return Err(ContextError::PathDenied(format!(
                "read target does not exist: {}",
                canonical.display()
            )));
        }
        if !inside(&canonical, worktree) {
            return Err(ContextError::PathDenied(format!(
                "{} escapes worktree {}",
                canonical.display(),
                worktree.display()
            )));
        }
        if effect != EffectClass::Read {
            let scoped = self
                .effect()
                .write_scopes
                .iter()
                .any(|scope| inside(&canonical, Path::new(scope)));
            if !scoped {
                return Err(ContextError::PathDenied(format!(
                    "{} is outside declared write scopes",
                    canonical.display()
                )));
            }
        }
        reject_hardlink(&canonical, effect)?;
        #[cfg(unix)]
        let identity = {
            let metadata = fs::metadata(&anchor).map_err(|error| io_error(&anchor, error))?;
            Some((metadata.dev(), metadata.ino()))
        };
        #[cfg(not(unix))]
        let identity = None;
        #[cfg(unix)]
        let worktree_identity = {
            let metadata = fs::metadata(worktree).map_err(|error| io_error(worktree, error))?;
            Some((metadata.dev(), metadata.ino()))
        };
        #[cfg(not(unix))]
        let worktree_identity = None;
        Ok(AuthorizedPath {
            canonical_path: canonical,
            anchor_path: anchor,
            anchor_identity: identity,
            existed,
            worktree_root: worktree.to_path_buf(),
            worktree_identity,
            write_scopes: self
                .effect()
                .write_scopes
                .iter()
                .map(PathBuf::from)
                .collect(),
            effect,
            live_context: self.clone(),
        })
    }
}
