use super::model::{EvidenceBinding, ProductFitnessError};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::Read;
use std::path::{Component, Path, PathBuf};

const MAX_EVIDENCE_BYTES: u64 = 64 * 1024 * 1024;

pub(super) fn validate(
    root: &Path,
    candidate_id: &str,
    evidence: &EvidenceBinding,
) -> Result<(), ProductFitnessError> {
    validate_digest(&evidence.sha256)?;
    validate_digest(&evidence.candidate_id)?;
    if evidence.candidate_id != candidate_id {
        return Err(ProductFitnessError::CandidateMismatch);
    }
    let root = checked_root(root)?;
    let path = root.join(checked_relative(&evidence.path)?);
    let metadata = fs::symlink_metadata(&path).map_err(|_| ProductFitnessError::EvidenceMissing)?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.len() > MAX_EVIDENCE_BYTES
        || hard_link_count(&metadata) != 1
    {
        return Err(ProductFitnessError::EvidenceSpecialFile);
    }
    let canonical = fs::canonicalize(&path).map_err(|_| ProductFitnessError::EvidenceMissing)?;
    if !canonical.starts_with(&root) || canonical != path {
        return Err(ProductFitnessError::EvidencePathInvalid);
    }
    let mut file = File::open(&path).map_err(|_| ProductFitnessError::EvidenceMissing)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|_| ProductFitnessError::EvidenceMissing)?;
    if format!("sha256:{:x}", Sha256::digest(&bytes)) != evidence.sha256 {
        return Err(ProductFitnessError::EvidenceDigestMismatch);
    }
    Ok(())
}

fn checked_root(root: &Path) -> Result<PathBuf, ProductFitnessError> {
    let metadata =
        fs::symlink_metadata(root).map_err(|_| ProductFitnessError::EvidencePathInvalid)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(ProductFitnessError::EvidencePathInvalid);
    }
    fs::canonicalize(root).map_err(|_| ProductFitnessError::EvidencePathInvalid)
}

fn checked_relative(value: &str) -> Result<PathBuf, ProductFitnessError> {
    if value.is_empty() || value.len() > 4096 || value.contains('\\') {
        return Err(ProductFitnessError::EvidencePathInvalid);
    }
    let path = Path::new(value);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(ProductFitnessError::EvidencePathInvalid);
    }
    Ok(path.to_path_buf())
}

#[cfg(unix)]
fn hard_link_count(metadata: &fs::Metadata) -> u64 {
    use std::os::unix::fs::MetadataExt;
    metadata.nlink()
}

#[cfg(not(unix))]
fn hard_link_count(_: &fs::Metadata) -> u64 {
    1
}

pub(super) fn validate_actor(value: &str) -> Result<(), ProductFitnessError> {
    if value.is_empty()
        || value.len() > 160
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_.:/".contains(&byte))
        || value.contains("..")
    {
        return Err(ProductFitnessError::InvalidActor);
    }
    Ok(())
}

pub(super) fn validate_digest(value: &str) -> Result<(), ProductFitnessError> {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return Err(ProductFitnessError::InvalidDigest);
    };
    if hex.len() != 64
        || !hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(ProductFitnessError::InvalidDigest);
    }
    Ok(())
}
