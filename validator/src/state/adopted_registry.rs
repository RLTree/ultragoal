use super::adopted_claims::AdoptedClaimRegistry;
use super::product_state::StateError;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

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

pub(super) struct LoadedAdoptedClaims {
    pub(super) handoff_sha256: String,
    pub(super) contract_manifest_sha256: String,
    pub(super) claim_registry_sha256: String,
    pub(super) registry: AdoptedClaimRegistry,
}

pub(super) fn load_adopted_claims() -> Result<LoadedAdoptedClaims, StateError> {
    load_claims_from(HANDOFF_BYTES, MANIFEST_BYTES, CLAIM_BYTES, HANDOFF_SHA256)
}

fn load_claims_from(
    handoff: &[u8],
    manifest_bytes: &[u8],
    claim_bytes: &[u8],
    expected_handoff_sha256: &str,
) -> Result<LoadedAdoptedClaims, StateError> {
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

    let mut registry: AdoptedClaimRegistry = serde_json::from_slice(claim_bytes)
        .map_err(|_| StateError::InvalidCatalog("adopted-claim-registry-invalid".to_owned()))?;
    validate_registry(&registry)?;
    canonicalize_registry(&mut registry);
    Ok(LoadedAdoptedClaims {
        handoff_sha256: format!("sha256:{expected_handoff_sha256}"),
        contract_manifest_sha256: format!("sha256:{manifest_digest}"),
        claim_registry_sha256: format!("sha256:{claim_digest}"),
        registry,
    })
}

fn validate_registry(registry: &AdoptedClaimRegistry) -> Result<(), StateError> {
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
    if !valid_prerequisites(registry) || registry.claims.iter().any(invalid_claim) {
        return invalid("adopted-claim-registry-semantics-mismatch");
    }
    Ok(())
}

fn canonicalize_registry(registry: &mut AdoptedClaimRegistry) {
    let positions = registry
        .claim_topological_order
        .iter()
        .enumerate()
        .map(|(index, id)| (id.as_str(), index))
        .collect::<BTreeMap<_, _>>();
    registry
        .claims
        .sort_by_key(|claim| positions.get(claim.claim_id.as_str()).copied());
}

fn valid_prerequisites(registry: &AdoptedClaimRegistry) -> bool {
    let positions = registry
        .claim_topological_order
        .iter()
        .enumerate()
        .map(|(index, id)| (id.as_str(), index))
        .collect::<BTreeMap<_, _>>();
    registry.claims.iter().all(|claim| {
        let Some(index) = positions.get(claim.claim_id.as_str()) else {
            return false;
        };
        claim.prerequisite_claim_ids.iter().all(|dependency| {
            positions
                .get(dependency.as_str())
                .is_some_and(|position| position < index)
        })
    })
}

fn invalid_claim(claim: &super::adopted_claims::AdoptedClaimDefinition) -> bool {
    let expected_owner = if matches!(claim.claim_id.as_str(), "CL-RELEASE" | "CL-COMPLETION") {
        "OWN-RELEASE-AUTHORITY"
    } else {
        "OWN-PROOF-AUTHORITY"
    };
    claim.name.is_empty()
        || claim.expected_behavior.is_empty()
        || claim.truth_surface.is_empty()
        || claim.requirement_ids.is_empty()
        || !valid_values(&claim.requirement_ids)
        || claim.required_surface_ids.is_empty()
        || !valid_values(&claim.required_surface_ids)
        || claim.required_tool_ids.is_empty()
        || !valid_values(&claim.required_tool_ids)
        || !valid_values(&claim.required_decision_ids)
        || claim.required_evidence.is_empty()
        || !valid_values(&claim.required_evidence)
        || claim.live_evidence_verification != "not_verified"
        || !claim.current_live_evidence.is_empty()
        || claim.independent_reconciler.is_empty()
        || claim.false_pass_controls.is_empty()
        || claim.claim_guard.is_empty()
        || claim.repair.is_empty()
        || claim.rerun.is_empty()
        || claim.allowed_ceiling_on_pass.is_empty()
        || claim.initial_claim_state != "not_evaluated_on_live_candidate"
        || claim.claim_decision_owner != expected_owner
}

fn valid_values(values: &[String]) -> bool {
    values.iter().all(|value| !value.trim().is_empty())
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
) -> Result<LoadedAdoptedClaims, StateError> {
    load_claims_from(handoff, manifest, claims, handoff_sha256)
}

#[cfg(test)]
pub(super) fn validate_registry_for_test(claims: &[u8]) -> Result<(), StateError> {
    let registry: AdoptedClaimRegistry = serde_json::from_slice(claims)
        .map_err(|_| StateError::InvalidCatalog("adopted-claim-registry-invalid".to_owned()))?;
    validate_registry(&registry)
}
