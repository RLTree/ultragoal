use super::digest::sha256_hex;
use super::types::{
    AuthorityCatalog, FindingSeverity, InventoryError, InventoryFinding, MAX_CATALOG_BYTES,
    ProjectionComparison,
};

impl AuthorityCatalog {
    pub fn compare_projection(
        &self,
        projection_bytes: &[u8],
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
                expected_sha256,
                actual_sha256,
                findings: Vec::new(),
            });
        }
        match serde_json::from_slice::<serde_json::Value>(projection_bytes) {
            Ok(_) => Ok(ProjectionComparison {
                matches: false,
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
