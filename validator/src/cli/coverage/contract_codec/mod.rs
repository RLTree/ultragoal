use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

mod manifest;
mod receipt;
mod scalar;

pub(crate) use manifest::CoverageManifest;
pub(crate) use receipt::{
    BoundaryLineage, CoverageMeasurement, CoverageReceipt, MachineReadableReport, TargetRevision,
    UncoveredRecord,
};
pub(crate) use scalar::{
    BlockedClaimClass, ClaimCeiling, ClaimId, CoverageCacheClass, CoverageCommandLine,
    CoveragePolicy, CoverageTargetDir, Digest, EquivalenceStatus, Reason, RepositoryPath, SchemaId,
    StrictBoundaryAuthorityStatus, SupportedClaimClass, TargetRevisionKind, ToolIdentity,
    ToolVersion, WorkspaceRoot,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CoverageReceiptMode {
    Strict,
    Routine,
}

pub(crate) enum CoverageContractRequest<'a> {
    ReadManifest {
        path: &'a Path,
    },
    ReadReceipt {
        path: &'a Path,
        mode: CoverageReceiptMode,
    },
    WriteReceipt {
        path: &'a Path,
        receipt: &'a CoverageReceipt,
    },
}

pub(crate) enum CoverageContractResponse {
    Manifest(CoverageManifest),
    Receipt(CoverageReceipt),
    ReceiptWritten,
}

#[derive(Debug)]
pub(crate) struct CoverageContractError {
    pub(crate) code: &'static str,
    pub(crate) path: PathBuf,
    pub(crate) detail: String,
}

impl fmt::Display for CoverageContractError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}:{}:{}",
            self.code,
            self.path.display(),
            self.detail
        )
    }
}

pub(crate) fn execute(
    request: CoverageContractRequest<'_>,
) -> Result<CoverageContractResponse, CoverageContractError> {
    match request {
        CoverageContractRequest::ReadManifest { path } => {
            let bytes = read_bytes(path, "coverage_manifest_read_failed")?;
            let manifest = serde_json::from_slice::<CoverageManifest>(&bytes).map_err(|error| {
                contract_error("coverage_manifest_contract_malformed", path, error)
            })?;
            Ok(CoverageContractResponse::Manifest(manifest))
        }
        CoverageContractRequest::ReadReceipt { path, mode } => {
            let bytes = read_bytes(path, "coverage_receipt_read_failed")?;
            let receipt = serde_json::from_slice::<CoverageReceipt>(&bytes).map_err(|error| {
                contract_error("coverage_receipt_contract_malformed", path, error)
            })?;
            reject_mode_mismatch(path, &receipt, mode)?;
            Ok(CoverageContractResponse::Receipt(receipt))
        }
        CoverageContractRequest::WriteReceipt { path, receipt } => {
            let bytes = serde_json::to_vec_pretty(receipt).map_err(|error| {
                contract_error("coverage_receipt_contract_encode_failed", path, error)
            })?;
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|error| {
                    contract_error("coverage_receipt_parent_create_failed", parent, error)
                })?;
            }
            let mut output = bytes;
            output.push(b'\n');
            fs::write(path, output).map_err(|error| {
                contract_error("coverage_receipt_contract_write_failed", path, error)
            })?;
            Ok(CoverageContractResponse::ReceiptWritten)
        }
    }
}

pub(crate) fn read_manifest(path: &Path) -> Result<CoverageManifest, CoverageContractError> {
    match execute(CoverageContractRequest::ReadManifest { path })? {
        CoverageContractResponse::Manifest(manifest) => Ok(manifest),
        _ => unreachable!("manifest request has one response type"),
    }
}

pub(crate) fn read_receipt(
    path: &Path,
    mode: CoverageReceiptMode,
) -> Result<CoverageReceipt, CoverageContractError> {
    match execute(CoverageContractRequest::ReadReceipt { path, mode })? {
        CoverageContractResponse::Receipt(receipt) => Ok(receipt),
        _ => unreachable!("receipt request has one response type"),
    }
}

pub(crate) fn write_receipt(
    path: &Path,
    receipt: &CoverageReceipt,
) -> Result<(), CoverageContractError> {
    execute(CoverageContractRequest::WriteReceipt { path, receipt }).map(|_| ())
}

pub(crate) fn lineage_digest(lineage: &BoundaryLineage) -> Result<Digest, CoverageContractError> {
    let value = serde_json::to_value(lineage).map_err(|error| {
        contract_error(
            "coverage_lineage_contract_encode_failed",
            Path::new("<coverage-lineage>"),
            error,
        )
    })?;
    Ok(Digest::new(crate::digest::canonical_json(&value)))
}

fn read_bytes(path: &Path, code: &'static str) -> Result<Vec<u8>, CoverageContractError> {
    fs::read(path).map_err(|error| contract_error(code, path, error))
}

fn reject_mode_mismatch(
    path: &Path,
    receipt: &CoverageReceipt,
    mode: CoverageReceiptMode,
) -> Result<(), CoverageContractError> {
    let routine_fields_present = receipt.coverage_cache_class.is_some()
        || receipt.equivalence_status.is_some()
        || receipt.boundary_lineage_digest.is_some();
    if mode == CoverageReceiptMode::Strict && routine_fields_present {
        return Err(contract_error(
            "coverage_strict_receipt_contains_routine_authority",
            path,
            "routine-only field present",
        ));
    }
    Ok(())
}

fn contract_error(
    code: &'static str,
    path: &Path,
    error: impl fmt::Display,
) -> CoverageContractError {
    CoverageContractError {
        code,
        path: path.to_path_buf(),
        detail: error.to_string(),
    }
}
