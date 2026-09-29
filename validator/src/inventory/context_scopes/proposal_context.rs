use crate::context::ReadSession;
use crate::inventory::digest::sha256_hex;
use crate::inventory::fs::read_bounded;
use crate::inventory::types::InventoryError;
use serde::Deserialize;
use std::path::Path;

const CONTEXT_ID: &str = "predecessor-plugin-proposal-report-2026-07-02";
const RELATIVE_PATH: &str = "REPORT.md";
const CONTENT_DIGEST: &str = "7638fbe3ac770524c5e8ef6e8ecbb0083de35c4c488d29d28a3861cec4d79a82";
const REPLACEMENT_CONTRACT: &str = "harness-ultragoal-successor-contract-v2";
const STATUS: &str = "replaced_historical_context";
const MAX_PROPOSAL_BYTES: u64 = 256 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ContextRow {
    context_id: String,
    path: String,
    sha256: String,
    authority: Authority,
    active_product_replaced: bool,
    replacement_contract_id: String,
    status: String,
    preserve: bool,
    physical_deletion_authorized: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Authority {
    binding: bool,
}

fn exact(row: &ContextRow) -> bool {
    row.context_id == CONTEXT_ID
        && row.path == RELATIVE_PATH
        && row.sha256 == CONTENT_DIGEST
        && !row.authority.binding
        && row.active_product_replaced
        && row.replacement_contract_id == REPLACEMENT_CONTRACT
        && row.status == STATUS
        && row.preserve
        && !row.physical_deletion_authorized
}

pub(super) fn registry_valid(rows: &[ContextRow]) -> bool {
    rows.is_empty() || rows.first().is_some_and(exact) && rows.len() == 1
}

pub(super) fn verify(
    reads: &ReadSession,
    repository_root: &Path,
    row: &ContextRow,
) -> Result<String, InventoryError> {
    if !exact(row) {
        return Err(InventoryError::InvalidRegistry(
            "predecessor proposal context row is invalid".to_owned(),
        ));
    }
    let bytes = read_bounded(
        reads,
        &repository_root.join(RELATIVE_PATH),
        MAX_PROPOSAL_BYTES,
    )?;
    if sha256_hex(&bytes) != CONTENT_DIGEST {
        return Err(InventoryError::InvalidRegistry(
            "predecessor proposal context digest is invalid".to_owned(),
        ));
    }
    Ok(RELATIVE_PATH.to_owned())
}

pub(super) const fn context_id() -> &'static str {
    CONTEXT_ID
}
