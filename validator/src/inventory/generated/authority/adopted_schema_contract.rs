use crate::context::ReadSession;
use crate::generated_authority::{RepositoryPath, Sha256Digest};
use crate::inventory::digest::file_identity_regular;
use crate::inventory::fs::read_bounded;
use crate::inventory::types::InventoryError;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::Path;

const MAX_BYTES: u64 = 1024 * 1024;
const ZERO_RECEIPT: &str =
    "sha256:0000000000000000000000000000000000000000000000000000000000000000";

pub(super) struct Binding<'a> {
    pub(super) output: &'a RepositoryPath,
    pub(super) sha256: &'a Sha256Digest,
    pub(super) schema: &'a RepositoryPath,
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
    regular_digest(reads, root, binding.schema)?;
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
    let text = std::str::from_utf8(&bytes)
        .map_err(|_| invalid_error("adopted schema amendment log invalid"))?;
    let rows = text
        .lines()
        .map(serde_json::from_str::<Value>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| invalid_error("adopted schema amendment log invalid"))?;
    let index = rows
        .iter()
        .position(|row| {
            row.get("amendment_id").and_then(Value::as_str) == Some(binding.amendment_id)
        })
        .ok_or_else(|| invalid_error("adopted schema amendment missing"))?;
    let row = &rows[index];
    let expected_hash = format!("sha256:{}", binding.amendment_hash.lowercase_hex());
    if row.get("amendment_hash").and_then(Value::as_str) != Some(&expected_hash)
        || row.get("new_contract_hash").and_then(Value::as_str)
            != Some(&format!("sha256:{source_digest}"))
        || !backlog_binds(row, binding.output.as_str(), output_digest)
        || !chain_is_valid(&rows, index)
        || canonical_hash(row)? != expected_hash
    {
        return invalid("adopted schema amendment binding invalid");
    }
    Ok(())
}

fn backlog_binds(row: &Value, output: &str, digest: &str) -> bool {
    row.get("backlog_updates")
        .and_then(Value::as_array)
        .is_some_and(|updates| {
            updates.iter().any(|update| {
                update.get("path").and_then(Value::as_str) == Some(output)
                    && update.get("digest").and_then(Value::as_str)
                        == Some(&format!("sha256:{digest}"))
            })
        })
}

fn chain_is_valid(rows: &[Value], index: usize) -> bool {
    let Some(row) = rows.get(index) else {
        return false;
    };
    if index == 0 {
        return true;
    }
    let Some(previous) = rows.get(index - 1) else {
        return false;
    };
    row.get("previous_amendment_hash") == previous.get("amendment_hash")
        && row.get("previous_contract_hash") == previous.get("new_contract_hash")
}

fn canonical_hash(row: &Value) -> Result<String, InventoryError> {
    let mut canonical = row.clone();
    canonical
        .as_object_mut()
        .ok_or_else(|| invalid_error("adopted schema amendment row invalid"))?
        .remove("amendment_hash");
    let bytes = serde_json::to_vec(&canonical)
        .map_err(|_| invalid_error("adopted schema amendment row invalid"))?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
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
