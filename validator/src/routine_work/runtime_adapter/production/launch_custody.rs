use super::*;

use std::fs;

use crate::routine_work::runtime_adapter::mediator::{ObjectIdentity, StagedProgram};

pub(super) struct EntryClaim {
    pub(super) path: PathBuf,
    pub(super) identity: ObjectIdentity,
    pub(super) bytes: Option<Vec<u8>>,
}

#[cfg(unix)]
pub(super) fn cleanup_created_child(
    child: &Path,
    identity: ObjectIdentity,
    _cause: std::io::Error,
    cause: &'static str,
) -> RoutineError {
    match cleanup_partial_stage(child, identity, &[]) {
        Ok(()) => error(cause),
        Err(cleanup_error) => cleanup_error,
    }
}

pub(super) fn cleanup_partial_stage(
    child: &Path,
    directory_identity: ObjectIdentity,
    claims: &[EntryClaim],
) -> Result<(), RoutineError> {
    let directory =
        fs::File::open(child).map_err(|_| error("routine-production-launch-directory-missing"))?;
    if ObjectIdentity::from(
        &directory
            .metadata()
            .map_err(|_| error("routine-production-launch-directory-stat-failed"))?,
    ) != directory_identity
    {
        return Err(error("routine-production-launch-directory-mismatch"));
    }
    let entries = fs::read_dir(child)
        .map_err(|_| error("routine-production-launch-directory-read-failed"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| error("routine-production-launch-directory-read-failed"))?;
    if entries.len() != claims.len()
        || entries.iter().any(|entry| {
            !claims
                .iter()
                .any(|claim| entry.file_name() == claim.path.file_name().unwrap_or_default())
        })
    {
        return Err(error(
            "routine-production-launch-directory-contents-invalid",
        ));
    }
    for claim in claims {
        let current = fs::symlink_metadata(&claim.path)
            .map_err(|_| error("routine-production-launch-entry-missing"))?;
        if ObjectIdentity::from(&current) != claim.identity || current.file_type().is_symlink() {
            return Err(error("routine-production-launch-entry-mismatch"));
        }
        if let Some(expected) = &claim.bytes
            && fs::read(&claim.path).ok().as_deref() != Some(expected.as_slice())
        {
            return Err(error("routine-production-launch-entry-bytes-mismatch"));
        }
    }
    for claim in claims.iter().rev() {
        fs::remove_file(&claim.path)
            .map_err(|_| error("routine-production-launch-entry-cleanup-failed"))?;
    }
    fs::remove_dir(child)
        .map_err(|_| error("routine-production-launch-directory-cleanup-failed"))?;
    Ok(())
}

pub(super) fn cleanup_staged(staged: &StagedProgram) -> Result<(), RoutineError> {
    cleanup_partial_stage(
        &staged.directory,
        staged.directory_identity,
        &[
            EntryClaim {
                path: staged.executable.path().to_path_buf(),
                identity: staged.executable.identity,
                bytes: None,
            },
            EntryClaim {
                path: staged.seal.clone(),
                identity: staged.seal_identity,
                bytes: Some(staged.seal_bytes.clone()),
            },
            EntryClaim {
                path: staged.marker.clone(),
                identity: staged.marker_identity,
                bytes: Some(staged.marker_bytes.clone()),
            },
        ],
    )
}
