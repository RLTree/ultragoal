use super::model::{EvidenceBinding, ProductFitnessError};
use sha2::{Digest, Sha256};
use std::path::Path;

#[cfg(unix)]
#[path = "evidence_filesystem.rs"]
mod filesystem;
#[cfg(not(unix))]
mod filesystem {
    use super::ProductFitnessError;
    use std::path::Path;

    pub(super) fn read_stable(_: &Path, _: &str) -> Result<Vec<u8>, ProductFitnessError> {
        Err(ProductFitnessError::EvidencePathInvalid)
    }
}

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
    let bytes = filesystem::read_stable(root, &evidence.path)?;
    if format!("sha256:{:x}", Sha256::digest(&bytes)) != evidence.sha256 {
        return Err(ProductFitnessError::EvidenceDigestMismatch);
    }
    Ok(())
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
