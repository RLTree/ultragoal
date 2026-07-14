use super::contract_codec::{CoverageReceiptMode, read_manifest, read_receipt};
use std::path::Path;

mod lineage_contract;
mod receipt_contract;

pub(super) fn failures(root: &Path, receipt: &Path, candidate: &str) -> Vec<String> {
    let path = match crate::output_path::claim_artifact_path(root, receipt, "coverage receipt") {
        Ok(path) => path,
        Err(error) => return vec![format!("coverage_routine_receipt_path_invalid:{error}")],
    };
    let receipt = match read_receipt(&path, CoverageReceiptMode::Routine) {
        Ok(receipt) => receipt,
        Err(error) => {
            return vec![format!(
                "coverage_routine_receipt_missing_or_malformed:{error}"
            )];
        }
    };
    let manifest_path = super::coverage_manifest_path(root);
    let manifest = match read_manifest(&manifest_path) {
        Ok(manifest) => manifest,
        Err(error) => {
            return vec![format!(
                "coverage_routine_manifest_missing_or_malformed:{error}"
            )];
        }
    };
    let mut out = Vec::new();
    receipt_contract::scalar_failures(root, &receipt, candidate, &mut out);
    lineage_contract::lineage_failures(root, &manifest, &receipt, &mut out);
    receipt_contract::claim_failures(&receipt, &mut out);
    out
}
