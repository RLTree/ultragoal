use super::contract_codec::{CoverageReceiptMode, read_manifest, read_receipt};
use std::path::Path;

mod manifest_contract;
mod receipt_contract;

pub(super) fn failures(root: &Path, receipt: &Path, candidate: &str) -> Vec<String> {
    let receipt_path =
        match crate::output_path::claim_artifact_path(root, receipt, "coverage receipt") {
            Ok(path) => path,
            Err(error) => return vec![format!("coverage_receipt_path_invalid:{error}")],
        };
    let receipt = match read_receipt(&receipt_path, CoverageReceiptMode::Strict) {
        Ok(receipt) => receipt,
        Err(error) => return vec![format!("coverage_receipt_missing_or_malformed:{error}")],
    };
    let manifest_path = super::coverage_manifest_path(root);
    let manifest = match read_manifest(&manifest_path) {
        Ok(manifest) => manifest,
        Err(error) => return vec![format!("coverage_manifest_missing_or_malformed:{error}")],
    };
    let mut out = Vec::new();
    receipt_contract::scalar_failures(root, &receipt, candidate, &mut out);
    manifest_contract::manifest_failures(root, &manifest, &mut out);
    manifest_contract::digest_failures(root, &manifest, &receipt, &mut out);
    receipt_contract::report_failures(root, &receipt, &mut out);
    receipt_contract::completion_failures(&receipt, &mut out);
    out
}
