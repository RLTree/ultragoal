use super::{MAX_OUTPUT_BYTES, Source, digest_hex};
use crate::generated_authority::{RepositoryPath, Sha256Digest};
use serde_json::Value;

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

pub(super) fn verify(source: &mut impl Source, binding: Binding<'_>) -> Result<(), String> {
    let output = source.read(binding.output.as_str(), MAX_OUTPUT_BYTES)?;
    let output_digest = digest_hex(output.as_ref());
    if output_digest != binding.sha256.lowercase_hex() {
        return Err("adopted schema contract output digest mismatch".to_string());
    }
    source.read(binding.schema.as_str(), MAX_OUTPUT_BYTES)?;
    let contract = source.read(binding.source_contract.as_str(), MAX_OUTPUT_BYTES)?;
    let source_digest = digest_hex(contract.as_ref());
    if source_digest != binding.source_contract_sha256.lowercase_hex() {
        return Err("adopted schema contract source digest mismatch".to_string());
    }
    let value: Value = serde_json::from_slice(output.as_ref())
        .map_err(|_| "adopted schema contract JSON invalid".to_string())?;
    let revision = value.get("target_revision").and_then(Value::as_object);
    if value.get("source_spec").is_some()
        || value.get("receipt_digest").and_then(Value::as_str)
            != Some("sha256:0000000000000000000000000000000000000000000000000000000000000000")
        || revision
            .and_then(|row| row.get("path"))
            .and_then(Value::as_str)
            != Some(binding.source_contract.as_str())
        || revision
            .and_then(|row| row.get("digest"))
            .and_then(Value::as_str)
            != Some(&format!("sha256:{source_digest}"))
    {
        return Err("adopted schema contract authority fields invalid".to_string());
    }
    verify_amendment(source, &binding, &output_digest, &source_digest)
}

fn verify_amendment(
    source: &mut impl Source,
    binding: &Binding<'_>,
    output_digest: &str,
    source_digest: &str,
) -> Result<(), String> {
    let bytes = source.read(binding.amendment_log.as_str(), MAX_OUTPUT_BYTES)?;
    let text = std::str::from_utf8(bytes.as_ref())
        .map_err(|_| "adopted schema amendment log invalid".to_string())?;
    let rows = text
        .lines()
        .map(serde_json::from_str::<Value>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "adopted schema amendment log invalid".to_string())?;
    let row = rows
        .iter()
        .find(|row| row.get("amendment_id").and_then(Value::as_str) == Some(binding.amendment_id))
        .ok_or_else(|| "adopted schema amendment missing".to_string())?;
    let expected_hash = format!("sha256:{}", binding.amendment_hash.lowercase_hex());
    let output_digest = format!("sha256:{output_digest}");
    let source_digest = format!("sha256:{source_digest}");
    let backlog_match = row
        .get("backlog_updates")
        .and_then(Value::as_array)
        .is_some_and(|updates| {
            updates.iter().any(|update| {
                update.get("path").and_then(Value::as_str) == Some(binding.output.as_str())
                    && update.get("digest").and_then(Value::as_str) == Some(&output_digest)
            })
        });
    if row.get("amendment_hash").and_then(Value::as_str) != Some(&expected_hash)
        || row.get("new_contract_hash").and_then(Value::as_str) != Some(&source_digest)
        || !backlog_match
    {
        return Err("adopted schema amendment binding invalid".to_string());
    }
    Ok(())
}
