use crate::context::ReadSession;
use crate::contract_amendment::{
    CurrentAmendmentBinding, ExpectedArtifactBinding, validate_current,
};
use crate::generated_authority::{RepositoryPath, Sha256Digest};
use crate::inventory::digest::file_identity_regular;
use crate::inventory::fs::read_bounded;
use crate::inventory::types::InventoryError;
use serde_json::Value;
use std::path::Path;

const MAX_BYTES: u64 = 1024 * 1024;
const ZERO_RECEIPT: &str =
    "sha256:0000000000000000000000000000000000000000000000000000000000000000";
const PREVIOUS_CONTRACT_HASH: &str =
    "sha256:6bd05cd382a2e8d1af10f6942ee64016f484983f5a98f118c4a3954ae8df6fa9";

pub(super) struct Binding<'a> {
    pub(super) output: &'a RepositoryPath,
    pub(super) sha256: &'a Sha256Digest,
    pub(super) schema: &'a RepositoryPath,
    pub(super) schema_sha256: &'a Sha256Digest,
    pub(super) source_contract: &'a RepositoryPath,
    pub(super) source_contract_sha256: &'a Sha256Digest,
    pub(super) amendment_log: &'a RepositoryPath,
    pub(super) amendment_id: &'a str,
    pub(super) amendment_hash: &'a Sha256Digest,
}

pub(super) fn verify(
    reads: &ReadSession,
    root: &Path,
    binding: Binding<'_>,
) -> Result<(), InventoryError> {
    let output_digest = regular_digest(reads, root, binding.output)?;
    if output_digest != binding.sha256.lowercase_hex() {
        return invalid("adopted schema contract output digest mismatch");
    }
    let schema_digest = regular_digest(reads, root, binding.schema)?;
    if schema_digest != binding.schema_sha256.lowercase_hex() {
        return invalid("adopted schema contract schema digest mismatch");
    }
    let source_digest = regular_digest(reads, root, binding.source_contract)?;
    if source_digest != binding.source_contract_sha256.lowercase_hex() {
        return invalid("adopted schema contract source digest mismatch");
    }
    verify_output(reads, root, &binding, &source_digest)?;
    verify_amendment(reads, root, &binding, &output_digest, &source_digest)
}

fn verify_output(
    reads: &ReadSession,
    root: &Path,
    binding: &Binding<'_>,
    source_digest: &str,
) -> Result<(), InventoryError> {
    let bytes = read_bounded(reads, &root.join(binding.output.as_str()), MAX_BYTES)?;
    let value: Value = serde_json::from_slice(&bytes)
        .map_err(|_| invalid_error("adopted schema contract JSON invalid"))?;
    let revision = value.get("target_revision").and_then(Value::as_object);
    if value.get("source_spec").is_some()
        || value.get("schema").and_then(Value::as_str)
            != Some("harness-ultragoal.product-success-contract.v1")
        || value.get("receipt_digest").and_then(Value::as_str) != Some(ZERO_RECEIPT)
        || revision
            .and_then(|row| row.get("path"))
            .and_then(Value::as_str)
            != Some(binding.source_contract.as_str())
        || revision
            .and_then(|row| row.get("digest"))
            .and_then(Value::as_str)
            != Some(&format!("sha256:{source_digest}"))
    {
        return invalid("adopted schema contract authority fields invalid");
    }
    Ok(())
}

fn verify_amendment(
    reads: &ReadSession,
    root: &Path,
    binding: &Binding<'_>,
    output_digest: &str,
    source_digest: &str,
) -> Result<(), InventoryError> {
    let bytes = read_bounded(reads, &root.join(binding.amendment_log.as_str()), MAX_BYTES)?;
    let expected_hash = format!("sha256:{}", binding.amendment_hash.lowercase_hex());
    validate_current(
        &bytes,
        CurrentAmendmentBinding {
            amendment_id: binding.amendment_id,
            amendment_hash: &expected_hash,
            previous_contract_hash: PREVIOUS_CONTRACT_HASH,
            new_contract_hash: &format!("sha256:{source_digest}"),
            change_class: "strengthens",
            backlog_updates: &[ExpectedArtifactBinding {
                path: binding.output.as_str(),
                digest: &format!("sha256:{output_digest}"),
            }],
        },
    )
    .map(|_| ())
    .map_err(|code| invalid_error(&format!("adopted schema amendment invalid: {code}")))
}

fn regular_digest(
    reads: &ReadSession,
    root: &Path,
    path: &RepositoryPath,
) -> Result<String, InventoryError> {
    file_identity_regular(reads, &root.join(path.as_str())).map(|(digest, _)| digest)
}

fn invalid(message: &str) -> Result<(), InventoryError> {
    Err(invalid_error(message))
}

fn invalid_error(message: &str) -> InventoryError {
    InventoryError::InvalidRegistry(message.to_owned())
}
