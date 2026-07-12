use super::digest::sha256_hex;
use super::types::{
    AuthorityCatalog, FindingSeverity, InventoryError, InventoryFinding, MAX_CATALOG_BYTES,
    ProjectionComparison,
};
use crate::context::LiveContext;
use std::fs;
use std::path::{Component, Path, PathBuf};

pub struct ProjectionCapture<'catalog> {
    catalog: &'catalog AuthorityCatalog,
    reads: crate::context::ReadSession,
    bytes: Vec<u8>,
}

impl ProjectionCapture<'_> {
    pub fn finish(self) -> Result<ProjectionComparison, InventoryError> {
        let comparison = self.catalog.compare_projection_bytes(&self.bytes, true)?;
        self.reads
            .revalidate()
            .map_err(|error| InventoryError::Context(error.to_string()))?;
        Ok(comparison)
    }
}

impl AuthorityCatalog {
    pub fn compare_projection(
        &self,
        projection_bytes: &[u8],
    ) -> Result<ProjectionComparison, InventoryError> {
        self.compare_projection_bytes(projection_bytes, false)
    }

    pub fn compare_projection_file(
        &self,
        context: &LiveContext,
        projection_path: impl AsRef<Path>,
    ) -> Result<ProjectionComparison, InventoryError> {
        self.capture_projection_file(context, projection_path)?
            .finish()
    }

    pub fn capture_projection_file<'catalog>(
        &'catalog self,
        context: &LiveContext,
        projection_path: impl AsRef<Path>,
    ) -> Result<ProjectionCapture<'catalog>, InventoryError> {
        if self.context_id() != context.context_id() {
            return Err(InventoryError::Context(
                "projection context does not match the catalog".to_owned(),
            ));
        }
        let path = confined_projection_path(context.worktree_root(), projection_path.as_ref())?;
        let metadata = fs::symlink_metadata(&path).map_err(|error| InventoryError::Io {
            path: path.clone(),
            message: error.to_string(),
        })?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(InventoryError::Activation(
                super::types::ActivationFailure::UnsafeInput,
            ));
        }
        let reads = context
            .begin_read_session()
            .map_err(|error| InventoryError::Context(error.to_string()))?;
        let bytes = reads
            .read_bounded(&path, MAX_CATALOG_BYTES as u64)
            .map_err(|error| InventoryError::Io {
                path: path.clone(),
                message: error.to_string(),
            })?;
        Ok(ProjectionCapture {
            catalog: self,
            reads,
            bytes,
        })
    }

    fn compare_projection_bytes(
        &self,
        projection_bytes: &[u8],
        input_verified_in_session: bool,
    ) -> Result<ProjectionComparison, InventoryError> {
        if projection_bytes.len() > MAX_CATALOG_BYTES {
            return Err(InventoryError::Serialization(format!(
                "projection exceeds {MAX_CATALOG_BYTES} bytes"
            )));
        }
        let expected_bytes = self.to_canonical_json()?;
        let expected_sha256 = sha256_hex(&expected_bytes);
        let actual_sha256 = sha256_hex(projection_bytes);
        if projection_bytes == expected_bytes {
            return Ok(ProjectionComparison {
                matches: true,
                input_verified_in_session,
                authority_eligible: false,
                expected_sha256,
                actual_sha256,
                findings: Vec::new(),
            });
        }
        match serde_json::from_slice::<serde_json::Value>(projection_bytes) {
            Ok(_) => Ok(ProjectionComparison {
                matches: false,
                input_verified_in_session,
                authority_eligible: false,
                expected_sha256,
                actual_sha256,
                findings: vec![InventoryFinding {
                    code: "stale_or_tampered_projection".to_owned(),
                    severity: FindingSeverity::Error,
                    entry_id: None,
                    relative_path: None,
                    message: "projection content differs from the context-bound catalog".to_owned(),
                }],
            }),
            Err(error) => Ok(ProjectionComparison {
                matches: false,
                input_verified_in_session,
                authority_eligible: false,
                expected_sha256,
                actual_sha256,
                findings: vec![InventoryFinding {
                    code: "invalid_projection_json".to_owned(),
                    severity: FindingSeverity::Error,
                    entry_id: None,
                    relative_path: None,
                    message: error.to_string(),
                }],
            }),
        }
    }
}

fn confined_projection_path(root: &Path, input: &Path) -> Result<PathBuf, InventoryError> {
    let path = if input.is_absolute() {
        input.to_path_buf()
    } else {
        root.join(input)
    };
    let relative = path
        .strip_prefix(root)
        .map_err(|_| InventoryError::PathEscape(path.clone()))?;
    if relative.as_os_str().is_empty()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(InventoryError::PathEscape(path));
    }
    Ok(path)
}
