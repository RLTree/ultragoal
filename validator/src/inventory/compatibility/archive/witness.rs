use super::specs::{ArchiveRouteSpec, by_archive_route_id};
use crate::context::ReadSession;
use crate::inventory::digest::sha256_hex;
use crate::inventory::fs::read_bounded;
use crate::inventory::routing_state::RouteTransition;
use crate::inventory::types::{ActiveStatus, AuthorityState, InventoryEntry, InventoryFinding};
use std::collections::BTreeSet;
use std::path::Path;

const DECISION_SHA256: &str = "41a727aa9ffcd3c6140a1bde3d88682dc72c5d35223b447ff5e4349766256639";
const LIB_PATH: &str = "validator/src/lib.rs";
const LIB_BYTES: &[u8] = include_bytes!("../../../lib.rs");

pub(crate) fn archive_proof_current(reads: &ReadSession, root: &Path) -> bool {
    read_bounded(reads, &root.join(super::specs::DECISION_PATH), 64 * 1024)
        .is_ok_and(|bytes| sha256_hex(&bytes) == DECISION_SHA256)
        && read_bounded(reads, &root.join(LIB_PATH), 2 * 1024 * 1024)
            .is_ok_and(|bytes| bytes == LIB_BYTES && !contains_claim_semantics_root(&bytes))
}

fn contains_claim_semantics_root(bytes: &[u8]) -> bool {
    bytes
        .windows(b"mod claim_semantics".len())
        .any(|window| window == b"mod claim_semantics")
}

pub(crate) fn archive_registry_route_is_compiled(
    route_id: &str,
    exact_stable_id: Option<&str>,
    canonical_target: &str,
    proof_refs: &[String],
) -> bool {
    by_archive_route_id(route_id).is_some_and(|route| {
        exact_stable_id == Some(route.stable_id)
            && canonical_target == route.canonical_target
            && route.proof_refs_match(proof_refs)
    })
}

fn target_is_exact(target: &InventoryEntry, spec: &ArchiveRouteSpec) -> bool {
    target.stable_id == spec.canonical_target
        && target.kind == spec.target_kind
        && target.relative_path == spec.target_path
        && target.digest_sha256 == spec.target_sha256
        && target.authority_state == AuthorityState::Canonical
        && target.active_status == ActiveStatus::Definition
        && target.generator.as_deref() == Some("HCT-INVENTORY:registry-loader")
}

fn target_has_conflict(target: &InventoryEntry, findings: &[InventoryFinding]) -> bool {
    findings.iter().any(|finding| {
        finding.entry_id.as_deref() == Some(target.stable_id.as_str())
            && matches!(
                finding.code.as_str(),
                "duplicate_stable_id" | "renamed_required_component" | "duplicate_component_path"
            )
    })
}

pub(crate) struct ArchiveRouteApplication<'a> {
    pub entry: &'a mut InventoryEntry,
    pub route_id: &'a str,
    pub exact_stable_id: Option<&'a str>,
    pub canonical_target: &'a str,
    pub proof_refs: &'a [String],
    pub transition: &'a RouteTransition,
    pub target: Option<&'a InventoryEntry>,
    pub archive_proof_is_current: bool,
    pub duplicate_stable_id_conflicts: &'a BTreeSet<String>,
    pub duplicate_path_conflicts: &'a BTreeSet<String>,
    pub findings: &'a mut Vec<InventoryFinding>,
}

pub(crate) fn apply_archive_route(request: ArchiveRouteApplication<'_>) -> bool {
    let ArchiveRouteApplication {
        entry,
        route_id,
        exact_stable_id,
        canonical_target,
        proof_refs,
        transition,
        target,
        archive_proof_is_current,
        duplicate_stable_id_conflicts,
        duplicate_path_conflicts,
        findings,
    } = request;
    let Some(spec) = by_archive_route_id(route_id) else {
        return false;
    };
    let no_conflicts = |id: &str| {
        !duplicate_stable_id_conflicts.contains(id) && !duplicate_path_conflicts.contains(id)
    };
    let target_current = target.is_some_and(|target| {
        target_is_exact(target, spec)
            && no_conflicts(&target.stable_id)
            && !target_has_conflict(target, findings)
    });
    let valid = exact_stable_id == Some(spec.stable_id)
        && canonical_target == spec.canonical_target
        && spec.proof_refs_match(proof_refs)
        && transition.verifies_retired_archive_transition()
        && entry.stable_id == spec.stable_id
        && entry.kind == spec.kind
        && entry.relative_path == spec.source_path
        && entry.digest_sha256 == spec.source_sha256
        && entry.authority_state == AuthorityState::Legacy
        && entry.active_status == ActiveStatus::Active
        && no_conflicts(&entry.stable_id)
        && archive_proof_is_current
        && target_current;
    if valid {
        entry.authority_state = AuthorityState::Context;
        entry.active_status = ActiveStatus::Retired;
        findings.push(InventoryFinding::info(
            "verified_od008_archive_context",
            Some(&entry.stable_id),
            Some(&entry.relative_path),
            "OD-008 preserves the exact dormant source as context-only; it has no production authority"
                .to_owned(),
        ));
    } else {
        findings.push(InventoryFinding::error(
            "invalid_od008_archive_transition",
            Some(&entry.stable_id),
            Some(&entry.relative_path),
            "OD-008 archive route failed exact source, target, reader, conflict, or transition proof"
                .to_owned(),
        ));
    }
    valid
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inventory::compatibility::archive::specs::DECISION_PATH;

    fn proof_refs(spec: &ArchiveRouteSpec) -> Vec<String> {
        vec![
            spec.source_path.to_owned(),
            spec.target_path.to_owned(),
            DECISION_PATH.to_owned(),
        ]
    }

    #[test]
    fn exact_od008_archive_route_is_compiled() {
        let spec =
            by_archive_route_id("lane-claim-dependency-to-ps-orchestration").expect("known route");
        assert!(archive_registry_route_is_compiled(
            spec.route_id,
            Some(spec.stable_id),
            spec.canonical_target,
            &proof_refs(spec),
        ));
    }

    #[test]
    fn altered_identity_or_decision_proof_cannot_match_archive_route() {
        let spec =
            by_archive_route_id("finalizer-ready-receipt-to-hct-claims").expect("known route");
        let proof = proof_refs(spec);
        assert!(!archive_registry_route_is_compiled(
            spec.route_id,
            Some("LEGACY-FINALIZER:forged"),
            spec.canonical_target,
            &proof,
        ));
        assert!(!archive_registry_route_is_compiled(
            spec.route_id,
            Some(spec.stable_id),
            "HCT-FORGED",
            &proof,
        ));
        let mut wrong_proof = proof;
        wrong_proof[2] = "docs/forged.json".to_owned();
        assert!(!archive_registry_route_is_compiled(
            spec.route_id,
            Some(spec.stable_id),
            spec.canonical_target,
            &wrong_proof,
        ));
    }
}
