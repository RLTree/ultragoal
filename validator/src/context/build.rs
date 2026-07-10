use super::capability;
use super::configuration;
use super::digest::sha256_hex;
use super::effects::permitted_effects;
use super::error::{ContextError, io_error};
use super::git;
use super::request::BuildRequest;
use super::types::{
    ContextPayload, EffectBoundary, EffectClass, LiveContext, PermissionIdentity, RootIdentity,
    SelectedInputIdentity,
};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::Read;
use std::path::{Component, Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

fn canonical(path: &Path) -> Result<PathBuf, ContextError> {
    path.canonicalize().map_err(|error| io_error(path, error))
}

pub(super) fn path_text(path: &Path) -> Result<String, ContextError> {
    path.to_str().map(ToOwned::to_owned).ok_or_else(|| {
        ContextError::InvalidRequest(format!("path is not UTF-8: {}", path.display()))
    })
}

fn match_expected(
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

fn reject_traversal(path: &Path) -> Result<(), ContextError> {
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

fn inside(path: &Path, root: &Path) -> bool {
    path == root || path.starts_with(root)
}

fn canonical_scopes(scopes: &[PathBuf], worktree: &Path) -> Result<Vec<String>, ContextError> {
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

fn mode(path: &Path) -> Result<Option<u32>, ContextError> {
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

pub(super) fn permissions(repository: &Path, worktree: &Path) -> PermissionIdentity {
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

fn digest_file(path: &Path) -> Result<(String, u64), ContextError> {
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

pub(super) fn selected_inputs(
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

impl LiveContext {
    pub fn build(request: BuildRequest) -> Result<Self, ContextError> {
        let grant = request.root_grant.as_ref();
        if request.effect != EffectClass::Read
            && grant.is_none_or(|grant| grant.effect() != request.effect)
        {
            return Err(ContextError::EffectDenied(format!(
                "selected {:?} effect lacks a matching root-issued grant",
                request.effect
            )));
        }
        let git_path = capability::resolve_git()?;
        let git_before = capability::executable_identity("git", &git_path)?;
        let (repository, worktree) = git::resolve_roots(&request.start, &git_path)?;
        match_expected(
            "repository",
            request.expected_repository_root.as_ref(),
            &repository,
        )?;
        match_expected(
            "worktree",
            request.expected_worktree_root.as_ref(),
            &worktree,
        )?;
        let scopes = canonical_scopes(
            grant.map(|grant| grant.write_scopes()).unwrap_or_default(),
            &worktree,
        )?;
        if matches!(
            request.effect,
            EffectClass::PlannedWrite | EffectClass::WorkspaceWrite | EffectClass::Destructive
        ) && scopes.is_empty()
        {
            return Err(ContextError::EffectDenied(
                "workspace-affecting context requires an explicit write scope".to_owned(),
            ));
        }
        let candidate = git::capture_candidate(&git_path, &worktree)?;
        let inputs = selected_inputs(&request.selected_inputs, &worktree)?;
        let capabilities = capability::capture(&request.tool_probes, &git_path)?;
        let permission_identity = permissions(&repository, &worktree);
        if candidate != git::capture_candidate(&git_path, &worktree)? {
            return Err(ContextError::ConcurrentMutation(
                "Git candidate identity".to_owned(),
            ));
        }
        if inputs != selected_inputs(&request.selected_inputs, &worktree)? {
            return Err(ContextError::ConcurrentMutation(
                "selected inputs".to_owned(),
            ));
        }
        if capabilities != capability::capture(&request.tool_probes, &git_path)?
            || git_before != capability::executable_identity("git", &git_path)?
        {
            return Err(ContextError::ConcurrentMutation(
                "tool capabilities, PATH, or Git substrate".to_owned(),
            ));
        }
        if permission_identity != permissions(&repository, &worktree) {
            return Err(ContextError::ConcurrentMutation(
                "root permissions".to_owned(),
            ));
        }
        let roots = RootIdentity {
            repository_root: path_text(&repository)?,
            worktree_root: path_text(&worktree)?,
        };
        let permitted = permitted_effects(request.effect);
        let payload = ContextPayload::new(
            roots,
            candidate,
            configuration::identity(&request.configuration, &request.secret_sources)?,
            capabilities,
            permission_identity,
            EffectBoundary {
                selected: request.effect,
                permitted,
                write_scopes: scopes,
            },
            inputs,
        );
        let serialized = serde_json::to_vec(&payload)
            .map_err(|error| ContextError::Serialization(error.to_string()))?;
        let context = Self::from_payload(payload, format!("sha256:{}", sha256_hex(&serialized)));
        context.revalidate()?;
        Ok(context)
    }

    pub fn to_canonical_json(&self) -> Result<Vec<u8>, ContextError> {
        serde_json::to_vec(self).map_err(|error| ContextError::Serialization(error.to_string()))
    }
}
