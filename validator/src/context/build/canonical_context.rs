use super::*;

pub(crate) fn canonical(path: &Path) -> Result<PathBuf, ContextError> {
    path.canonicalize().map_err(|error| io_error(path, error))
}

pub(crate) fn path_text(path: &Path) -> Result<String, ContextError> {
    path.to_str().map(ToOwned::to_owned).ok_or_else(|| {
        ContextError::InvalidRequest(format!("path is not UTF-8: {}", path.display()))
    })
}

pub(crate) fn match_expected(
    dimension: &'static str,
    expected: Option<&PathBuf>,
    actual: &Path,
) -> Result<(), ContextError> {
    if let Some(expected) = expected {
        let expected = canonical(expected)?;
        if expected != actual {
            return Err(ContextError::RootMismatch {
                dimension,
                expected,
                actual: actual.to_path_buf(),
            });
        }
    }
    Ok(())
}

pub(crate) fn reject_traversal(path: &Path) -> Result<(), ContextError> {
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

pub(crate) fn inside(path: &Path, root: &Path) -> bool {
    path == root || path.starts_with(root)
}

pub(crate) fn canonical_scopes(
    scopes: &[PathBuf],
    worktree: &Path,
) -> Result<Vec<String>, ContextError> {
    let mut canonical_paths = Vec::with_capacity(scopes.len());
    for scope in scopes {
        reject_traversal(scope)?;
        let joined = if scope.is_absolute() {
            scope.clone()
        } else {
            worktree.join(scope)
        };
        let resolved = canonical(&joined)?;
        if !resolved.is_dir() || !inside(&resolved, worktree) {
            return Err(ContextError::PathDenied(format!(
                "write scope must be an existing directory inside the worktree: {}",
                resolved.display()
            )));
        }
        canonical_paths.push(path_text(&resolved)?);
    }
    canonical_paths.sort();
    canonical_paths.dedup();
    Ok(canonical_paths)
}

pub(crate) fn mode(path: &Path) -> Result<Option<u32>, ContextError> {
    let metadata = fs::metadata(path).map_err(|error| io_error(path, error))?;
    #[cfg(unix)]
    {
        Ok(Some(metadata.permissions().mode()))
    }
    #[cfg(not(unix))]
    {
        let _ = metadata;
        Ok(None)
    }
}

pub(crate) fn permissions(repository: &Path, worktree: &Path) -> PermissionIdentity {
    PermissionIdentity {
        repository_metadata_read_succeeded: fs::metadata(repository).is_ok(),
        worktree_metadata_read_succeeded: fs::metadata(worktree).is_ok(),
        repository_directory_open_succeeded: fs::read_dir(repository).is_ok(),
        worktree_directory_open_succeeded: fs::read_dir(worktree).is_ok(),
        repository_unix_mode: mode(repository).ok().flatten(),
        worktree_unix_mode: mode(worktree).ok().flatten(),
        write_probe_performed: false,
    }
}

pub(crate) fn digest_file(path: &Path) -> Result<(String, u64), ContextError> {
    let metadata = fs::metadata(path).map_err(|error| io_error(path, error))?;
    if !metadata.is_file() {
        return Err(ContextError::InvalidRequest(format!(
            "selected input is not a regular file: {}",
            path.display()
        )));
    }
    let mut file = File::open(path).map_err(|error| io_error(path, error))?;
    let mut hasher = Sha256::new();
    let mut length = 0_u64;
    let mut buffer = [0_u8; 16 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| io_error(path, error))?;
        if read == 0 {
            break;
        }
        length += read as u64;
        hasher.update(&buffer[..read]);
    }
    Ok((format!("{:x}", hasher.finalize()), length))
}

pub(crate) fn selected_inputs(
    paths: &[PathBuf],
    worktree: &Path,
) -> Result<Vec<SelectedInputIdentity>, ContextError> {
    let mut identities = Vec::with_capacity(paths.len());
    for path in paths {
        reject_traversal(path)?;
        let joined = if path.is_absolute() {
            path.clone()
        } else {
            worktree.join(path)
        };
        let resolved = canonical(&joined)?;
        if !inside(&resolved, worktree) {
            return Err(ContextError::PathDenied(format!(
                "selected input escapes worktree: {}",
                resolved.display()
            )));
        }
        let relative = resolved.strip_prefix(worktree).expect("inside worktree");
        let (sha256, byte_length) = digest_file(&resolved)?;
        identities.push(SelectedInputIdentity {
            relative_path: path_text(relative)?,
            sha256,
            byte_length,
            unix_mode: mode(&resolved)?,
        });
    }
    identities.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    if identities
        .windows(2)
        .any(|pair| pair[0].relative_path == pair[1].relative_path)
    {
        return Err(ContextError::InvalidRequest(
            "duplicate selected input".to_owned(),
        ));
    }
    Ok(identities)
}
