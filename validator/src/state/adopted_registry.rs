use super::catalog::ClaimSpec;
use super::types::StateError;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

const CONTRACT_ID: &str = "harness-ultragoal-successor-contract-v2";
const HANDOFF_SHA256: &str = "d61c897a68d3aa985996f595a17c80f49e0730d07434b6b81de36878ef28dc51";
const CLAIM_PATH: &str = "FINAL-CONTRACT/CLAIM_REGISTRY.json";
const MANIFEST_PATH: &str = "FINAL-CONTRACT/CONTRACT_MANIFEST.json";
const HANDOFF_BYTES: &[u8] = include_bytes!(
    "../../../docs/ultragoal-contract-2026-07-successor-v2/FINAL-HANDOFF-MANIFEST.sha256"
);
const MANIFEST_BYTES: &[u8] = include_bytes!(
    "../../../docs/ultragoal-contract-2026-07-successor-v2/FINAL-CONTRACT/CONTRACT_MANIFEST.json"
);
const CLAIM_BYTES: &[u8] = include_bytes!(
    "../../../docs/ultragoal-contract-2026-07-successor-v2/FINAL-CONTRACT/CLAIM_REGISTRY.json"
);

#[derive(Deserialize)]
struct ContractManifest {
    contract_id: String,
    contract_entries: Vec<ContractEntry>,
}

#[derive(Deserialize)]
struct ContractEntry {
    path: String,
    sha256: String,
    bytes: usize,
    normative: bool,
}

#[derive(Deserialize)]
struct ClaimRegistry {
    schema_version: String,
    contract_id: String,
    claim_count: usize,
    claim_topological_order: Vec<String>,
    claims: Vec<ClaimRow>,
}

#[derive(Deserialize)]
struct ClaimRow {
    claim_id: String,
    allowed_ceiling_on_pass: String,
}

pub(super) fn load_adopted_claims() -> Result<(String, Vec<ClaimSpec>), StateError> {
    load_claims_from(HANDOFF_BYTES, MANIFEST_BYTES, CLAIM_BYTES, HANDOFF_SHA256)
}

fn load_claims_from(
    handoff: &[u8],
    manifest_bytes: &[u8],
    claim_bytes: &[u8],
    expected_handoff_sha256: &str,
) -> Result<(String, Vec<ClaimSpec>), StateError> {
    if sha256(handoff) != expected_handoff_sha256 {
        return invalid("adopted-handoff-identity-mismatch");
    }
    let manifest_digest = handoff_digest(handoff, MANIFEST_PATH)?;
    if sha256(manifest_bytes) != manifest_digest {
        return invalid("adopted-contract-manifest-identity-mismatch");
    }
    let claim_digest = handoff_digest(handoff, CLAIM_PATH)?;
    if sha256(claim_bytes) != claim_digest {
        return invalid("adopted-claim-registry-identity-mismatch");
    }

    let manifest: ContractManifest = serde_json::from_slice(manifest_bytes)
        .map_err(|_| StateError::InvalidCatalog("adopted-contract-manifest-invalid".to_owned()))?;
    if manifest.contract_id != CONTRACT_ID {
        return invalid("adopted-contract-id-mismatch");
    }
    let rows = manifest
        .contract_entries
        .iter()
        .filter(|entry| entry.path == CLAIM_PATH)
        .collect::<Vec<_>>();
    if rows.len() != 1
        || rows[0].sha256 != claim_digest
        || rows[0].bytes != claim_bytes.len()
        || !rows[0].normative
    {
        return invalid("adopted-claim-registry-manifest-row-mismatch");
    }

    let registry: ClaimRegistry = serde_json::from_slice(claim_bytes)
        .map_err(|_| StateError::InvalidCatalog("adopted-claim-registry-invalid".to_owned()))?;
    if registry.schema_version != "2.0.0"
        || registry.contract_id != CONTRACT_ID
        || registry.claim_count != registry.claims.len()
        || registry.claim_count != registry.claim_topological_order.len()
    {
        return invalid("adopted-claim-registry-shape-mismatch");
    }
    let order = registry
        .claim_topological_order
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let ids = registry
        .claims
        .iter()
        .map(|claim| claim.claim_id.clone())
        .collect::<BTreeSet<_>>();
    if order.len() != registry.claim_count || ids != order {
        return invalid("adopted-claim-registry-order-mismatch");
    }
    let claims = registry
        .claims
        .into_iter()
        .map(|claim| ClaimSpec {
            claim_id: claim.claim_id,
            maximum_dimensions: vec![claim.allowed_ceiling_on_pass],
        })
        .collect();
    Ok((format!("sha256:{claim_digest}"), claims))
}

fn handoff_digest(bytes: &[u8], wanted: &str) -> Result<String, StateError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| StateError::InvalidCatalog("adopted-handoff-invalid".to_owned()))?;
    let rows = text
        .lines()
        .filter_map(|line| line.split_once("  "))
        .filter(|(_, path)| *path == wanted)
        .collect::<Vec<_>>();
    if rows.len() != 1 || !valid_hex(rows[0].0) {
        return invalid("adopted-handoff-row-mismatch");
    }
    Ok(rows[0].0.to_owned())
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn valid_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn invalid<T>(code: &str) -> Result<T, StateError> {
    Err(StateError::InvalidCatalog(code.to_owned()))
}

#[cfg(test)]
pub(super) fn load_claims_for_test(
    handoff: &[u8],
    manifest: &[u8],
    claims: &[u8],
    handoff_sha256: &str,
) -> Result<(String, Vec<ClaimSpec>), StateError> {
    load_claims_from(handoff, manifest, claims, handoff_sha256)
}
