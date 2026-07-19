use super::{MIGRATION_REGISTRY_PATH, RoutingData};
use crate::inventory::compatibility::{
    AgentRouteApplication, RETAINED_KIND, apply_agent_route, apply_route, retention_candidates,
};
use crate::inventory::types::{AuthorityState, InventoryEntry, InventoryFinding};
use std::collections::{BTreeMap, BTreeSet};

impl RoutingData {
    pub(crate) fn apply(
        &self,
        entries: &mut BTreeMap<String, InventoryEntry>,
        findings: &mut Vec<InventoryFinding>,
        duplicate_stable_id_conflicts: &BTreeSet<String>,
        duplicate_path_conflicts: &BTreeSet<String>,
    ) {
        let present_skills = retention_candidates(entries, findings);
        let targets = entries.clone();
        for entry in entries
            .values_mut()
            .filter(|entry| entry.authority_state == AuthorityState::Legacy)
        {
            let matches = self
                .registry
                .routes
                .iter()
                .filter(|route| route.matcher.matches(entry))
                .collect::<Vec<_>>();
            match matches.as_slice() {
                [] => findings.push(InventoryFinding::error(
                    "unrouted_legacy_authority",
                    Some(&entry.stable_id),
                    Some(&entry.relative_path),
                    "legacy surface has no adopted authority disposition".to_owned(),
                )),
                [route] => {
                    let requests_retention =
                        route.transition.requests_retained_compatibility_route();
                    let endpoint_stable_ids_unique = !duplicate_stable_id_conflicts
                        .contains(&entry.stable_id)
                        && !duplicate_stable_id_conflicts.contains(&route.canonical_target);
                    apply_route(
                        entry,
                        &route.route_id,
                        route.matcher.exact_stable_id(),
                        &route.canonical_target,
                        requests_retention,
                        endpoint_stable_ids_unique
                            && present_skills.contains(&route.canonical_target),
                        findings,
                    );
                    let retained = requests_retention && entry.kind == RETAINED_KIND;
                    let agent_verified = apply_agent_route(AgentRouteApplication {
                        entry,
                        route_id: &route.route_id,
                        exact_stable_id: route.matcher.exact_stable_id(),
                        canonical_target: &route.canonical_target,
                        proof_refs: &route.transition.proof_refs,
                        transition: &route.transition,
                        target: targets.get(&route.canonical_target),
                        reader_proof_is_current: self.reader_proof_is_current,
                        duplicate_stable_id_conflicts,
                        duplicate_path_conflicts,
                        findings,
                    });
                    entry.input_provenance.push(format!(
                        "{MIGRATION_REGISTRY_PATH}#/routes/{}",
                        route.route_id
                    ));
                    if retained || agent_verified {
                        entry
                            .input_provenance
                            .extend(route.transition.proof_refs.iter().cloned());
                    }
                    entry.references.push(route.canonical_target.clone());
                    entry.normalize();
                }
                _ => findings.push(InventoryFinding::error(
                    "ambiguous_authority_route",
                    Some(&entry.stable_id),
                    Some(&entry.relative_path),
                    format!("legacy surface matches {} route rules", matches.len()),
                )),
            }
        }
    }
}
