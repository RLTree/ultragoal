use super::agent_specs::{AgentRouteSpec, TargetState, by_agent_route_id};
use crate::inventory::routing_state::RouteTransition;
use crate::inventory::types::{ActiveStatus, AuthorityState, InventoryEntry, InventoryFinding};
use std::collections::BTreeSet;
pub(crate) fn agent_registry_route_is_compiled(
    route_id: &str,
    exact_stable_id: Option<&str>,
    canonical_target: &str,
    proof_refs: &[String],
) -> bool {
    by_agent_route_id(route_id).is_some_and(|route| {
        exact_stable_id == Some(route.stable_id().as_str())
            && canonical_target == route.canonical_target
            && route.proof_refs_match(proof_refs)
    })
}

fn target_state_matches(target: &InventoryEntry, spec: &AgentRouteSpec) -> bool {
    let common = target.stable_id == spec.canonical_target
        && target.relative_path == spec.target_path
        && target.digest_sha256 == spec.target_sha256
        && target.authority_state == AuthorityState::Canonical;
    common
        && match spec.target_state {
            TargetState::ActiveAgent => {
                target.kind == "agent"
                    && target.owner_role == "OWN-ULTRA-ROOT"
                    && target.active_status == ActiveStatus::Active
                    && target.generator.is_none()
            }
            TargetState::AdoptedProductDefinition => {
                target.kind == "product-surface-definition"
                    && target.active_status == ActiveStatus::Definition
                    && target.generator.as_deref() == Some("HCT-INVENTORY:registry-loader")
            }
        }
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

pub(crate) struct AgentRouteApplication<'a> {
    pub entry: &'a mut InventoryEntry,
    pub route_id: &'a str,
    pub exact_stable_id: Option<&'a str>,
    pub canonical_target: &'a str,
    pub proof_refs: &'a [String],
    pub transition: &'a RouteTransition,
    pub target: Option<&'a InventoryEntry>,
    pub reader_proof_is_current: bool,
    pub duplicate_stable_id_conflicts: &'a BTreeSet<String>,
    pub duplicate_path_conflicts: &'a BTreeSet<String>,
    pub findings: &'a mut Vec<InventoryFinding>,
}

pub(crate) fn apply_agent_route(request: AgentRouteApplication<'_>) -> bool {
    let AgentRouteApplication {
        entry,
        route_id,
        exact_stable_id,
        canonical_target,
        proof_refs,
        transition,
        target,
        reader_proof_is_current,
        duplicate_stable_id_conflicts,
        duplicate_path_conflicts,
        findings,
    } = request;
    let Some(spec) = by_agent_route_id(route_id) else {
        return false;
    };
    if !transition.claims_demoted() {
        return false;
    }
    let target_current = target.is_some_and(|target| {
        target_state_matches(target, spec)
            && !duplicate_stable_id_conflicts.contains(&target.stable_id)
            && !duplicate_path_conflicts.contains(&target.stable_id)
            && !target_has_conflict(target, findings)
    });
    let verified = exact_stable_id == Some(spec.stable_id().as_str())
        && canonical_target == spec.canonical_target
        && spec.proof_refs_match(proof_refs)
        && transition.verifies_agent_context_transition()
        && entry.stable_id == spec.stable_id()
        && entry.kind == "legacy-agent-authority"
        && entry.relative_path == spec.legacy_path
        && entry.digest_sha256 == spec.legacy_sha256
        && entry.authority_state == AuthorityState::Legacy
        && entry.active_status == ActiveStatus::Active
        && !duplicate_stable_id_conflicts.contains(&entry.stable_id)
        && !duplicate_path_conflicts.contains(&entry.stable_id)
        && reader_proof_is_current
        && target_current;
    if verified {
        entry.authority_state = AuthorityState::Context;
        entry.active_status = ActiveStatus::ContextOnly;
        findings.push(InventoryFinding::warning(
            "verified_agent_route_context",
            Some(&entry.stable_id),
            Some(&entry.relative_path),
            "exact candidate-bound agent route verified; preserved descriptor is context-only"
                .to_owned(),
        ));
    } else {
        findings.push(InventoryFinding::error(
            "invalid_agent_route_transition",
            Some(&entry.stable_id),
            Some(&entry.relative_path),
            "agent route failed its compiled source, target, reader, or preservation witness"
                .to_owned(),
        ));
    }
    verified
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(spec: &AgentRouteSpec, target: bool) -> InventoryEntry {
        InventoryEntry {
            stable_id: if target {
                spec.canonical_target.to_owned()
            } else {
                spec.stable_id()
            },
            kind: if target {
                "agent"
            } else {
                "legacy-agent-authority"
            }
            .to_owned(),
            owner_role: "OWN-ULTRA-ROOT".to_owned(),
            relative_path: if target {
                spec.target_path
            } else {
                spec.legacy_path
            }
            .to_owned(),
            digest_sha256: if target {
                spec.target_sha256
            } else {
                spec.legacy_sha256
            }
            .to_owned(),
            unix_mode: None,
            authority_state: if target {
                AuthorityState::Canonical
            } else {
                AuthorityState::Legacy
            },
            active_status: ActiveStatus::Active,
            generator: None,
            input_provenance: Vec::new(),
            references: Vec::new(),
        }
    }

    fn transition(proof_refs: &[String]) -> RouteTransition {
        serde_json::from_value(serde_json::json!({
            "compatibility_behavior": "not-applicable",
            "compatibility_boundary": "adopted",
            "replacement_state": "candidate-required",
            "active_reader_writer_state": "none-verified",
            "observed_authority_state": "context-only",
            "equivalence_proof": "not-applicable",
            "physical_cleanup_state": "preserve",
            "proof_refs": proof_refs,
        }))
        .unwrap()
    }

    #[test]
    fn duplicate_source_or_target_path_blocks_legacy_demotion() {
        let spec = by_agent_route_id("agent-contract-claim-falsifier-to-claim-falsifier").unwrap();
        let target = row(spec, true);
        let proof_refs = [
            spec.legacy_path.to_owned(),
            spec.target_path.to_owned(),
            super::super::agent_specs::READER_PROOF_PATH.to_owned(),
        ];
        let transition = transition(&proof_refs);
        for conflict in [spec.stable_id(), spec.canonical_target.to_owned()] {
            let mut legacy = row(spec, false);
            let mut findings = Vec::new();
            assert!(!apply_agent_route(AgentRouteApplication {
                entry: &mut legacy,
                route_id: spec.route_id,
                exact_stable_id: Some(&spec.stable_id()),
                canonical_target: spec.canonical_target,
                proof_refs: &proof_refs,
                transition: &transition,
                target: Some(&target),
                reader_proof_is_current: true,
                duplicate_stable_id_conflicts: &BTreeSet::new(),
                duplicate_path_conflicts: &BTreeSet::from([conflict]),
                findings: &mut findings,
            }));
            assert_eq!(legacy.authority_state, AuthorityState::Legacy);
            assert_eq!(legacy.active_status, ActiveStatus::Active);
            assert!(
                findings
                    .iter()
                    .any(|finding| finding.code == "invalid_agent_route_transition")
            );
        }
    }

    #[test]
    fn duplicate_source_or_target_stable_id_blocks_legacy_demotion() {
        let spec = by_agent_route_id("agent-contract-claim-falsifier-to-claim-falsifier").unwrap();
        let target = row(spec, true);
        let proof_refs = [
            spec.legacy_path.to_owned(),
            spec.target_path.to_owned(),
            super::super::agent_specs::READER_PROOF_PATH.to_owned(),
        ];
        let transition = transition(&proof_refs);
        for conflict in [spec.stable_id(), spec.canonical_target.to_owned()] {
            let mut legacy = row(spec, false);
            assert!(!apply_agent_route(AgentRouteApplication {
                entry: &mut legacy,
                route_id: spec.route_id,
                exact_stable_id: Some(&spec.stable_id()),
                canonical_target: spec.canonical_target,
                proof_refs: &proof_refs,
                transition: &transition,
                target: Some(&target),
                reader_proof_is_current: true,
                duplicate_stable_id_conflicts: &BTreeSet::from([conflict]),
                duplicate_path_conflicts: &BTreeSet::new(),
                findings: &mut Vec::new(),
            }));
            assert_eq!(legacy.authority_state, AuthorityState::Legacy);
        }
    }
}
