use serde_json::Value;
use std::path::{Path, PathBuf};

pub(in crate::cli::typed_boundaries) struct ArtifactWrite {
    pub(in crate::cli::typed_boundaries) digest: String,
    pub(in crate::cli::typed_boundaries) status: String,
}

pub(in crate::cli::typed_boundaries) struct ClaimReceiptPath {
    absolute: PathBuf,
}

impl ClaimReceiptPath {
    pub(in crate::cli::typed_boundaries) fn new(
        root: &Path,
        receipt: &Path,
    ) -> Result<Self, String> {
        let absolute =
            crate::output_path::claim_artifact_path(root, receipt, "typed boundaries receipt")?;
        Ok(Self { absolute })
    }

    fn as_path(&self) -> &Path {
        &self.absolute
    }
}

pub(in crate::cli::typed_boundaries) fn write_receipt(
    receipt: &ClaimReceiptPath,
    value: &Value,
) -> Result<(), String> {
    crate::json_boundary::write_json(receipt.as_path(), value)
}

pub(in crate::cli::typed_boundaries) fn write_inventory_artifact(
    root: &Path,
    rel: &'static str,
    value: &Value,
    label: &str,
) -> Result<ArtifactWrite, String> {
    let path = crate::output_path::literal_claim_artifact_path(root, rel, label);
    let text =
        serde_json::to_string_pretty(value).expect("serde_json::Value serialization is infallible");
    let payload = format!("{text}\n");
    let digest = crate::digest::bytes(payload.as_bytes());
    crate::output_path::prepare_parent(&path)?;
    if crate::digest::file(&path).ok().as_deref() == Some(digest.as_str()) {
        return Ok(ArtifactWrite {
            digest,
            status: "reused_current_artifact".to_string(),
        });
    }
    crate::output_path::write(&path, payload, label)?;
    Ok(ArtifactWrite {
        digest,
        status: "published_current_artifact".to_string(),
    })
}
