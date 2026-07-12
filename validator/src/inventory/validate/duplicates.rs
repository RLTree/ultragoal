use crate::inventory::types::{InventoryEntry, InventoryFinding};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn duplicate_paths(
    groups: &[Vec<InventoryEntry>],
    findings: &mut Vec<InventoryFinding>,
) -> BTreeSet<String> {
    let mut paths: BTreeMap<&str, Vec<&InventoryEntry>> = BTreeMap::new();
    for entry in groups.iter().flatten() {
        if entry.relative_path.contains("#/") || entry.kind == "source-symbol-implementation" {
            continue;
        }
        paths.entry(&entry.relative_path).or_default().push(entry);
    }
    let mut conflicts = BTreeSet::new();
    for (path, owners) in paths
        .into_iter()
        .filter(|(_, owners)| owners.len() > 1 && !expected_required_actual_pair(owners))
    {
        conflicts.extend(owners.iter().map(|entry| entry.stable_id.clone()));
        let prior = owners[0].stable_id.as_str();
        for owner in owners.into_iter().skip(1) {
            findings.push(InventoryFinding::error(
                "duplicate_component_path",
                Some(&owner.stable_id),
                Some(path),
                format!("path is also owned by {prior}"),
            ));
        }
    }
    conflicts
}

fn expected_required_actual_pair(entries: &[&InventoryEntry]) -> bool {
    if entries.len() != 2 || entries[0].stable_id != entries[1].stable_id {
        return false;
    }
    let required = |entry: &InventoryEntry| {
        matches!(
            entry.active_status,
            crate::inventory::types::ActiveStatus::Required
                | crate::inventory::types::ActiveStatus::Missing
        )
    };
    required(entries[0]) ^ required(entries[1])
}

pub(super) fn duplicate_stable_ids(groups: &[Vec<InventoryEntry>]) -> BTreeSet<String> {
    let mut observed_required = BTreeMap::new();
    let mut conflicts = BTreeSet::new();
    for entry in groups.iter().flatten() {
        let incoming_required = matches!(
            entry.active_status,
            crate::inventory::types::ActiveStatus::Required
                | crate::inventory::types::ActiveStatus::Missing
        );
        match observed_required.get_mut(&entry.stable_id) {
            None => {
                observed_required.insert(entry.stable_id.clone(), incoming_required);
            }
            Some(existing_required) if *existing_required ^ incoming_required => {
                *existing_required = false;
            }
            Some(_) => {
                conflicts.insert(entry.stable_id.clone());
            }
        }
    }
    conflicts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inventory::types::{ActiveStatus, AuthorityState};

    fn entry(id: &str) -> InventoryEntry {
        InventoryEntry {
            stable_id: id.to_owned(),
            kind: "agent".to_owned(),
            owner_role: "OWN-ULTRA-ROOT".to_owned(),
            relative_path: "same/path.toml".to_owned(),
            digest_sha256: "digest".to_owned(),
            unix_mode: None,
            authority_state: AuthorityState::Canonical,
            active_status: ActiveStatus::Active,
            generator: None,
            input_provenance: Vec::new(),
            references: Vec::new(),
        }
    }

    #[test]
    fn duplicate_path_conflict_set_contains_every_owner() {
        let groups = vec![vec![
            entry("AGENT:claim-falsifier"),
            entry("ALIAS:claim-falsifier"),
        ]];
        let mut findings = Vec::new();
        let conflicts = duplicate_paths(&groups, &mut findings);
        assert_eq!(
            conflicts,
            BTreeSet::from([
                "AGENT:claim-falsifier".to_owned(),
                "ALIAS:claim-falsifier".to_owned(),
            ])
        );
        assert_eq!(
            findings
                .iter()
                .filter(|finding| finding.code == "duplicate_component_path")
                .count(),
            1
        );
    }

    #[test]
    fn raw_duplicate_path_survives_stable_id_merge_collapse() {
        let duplicate = entry("AGENT:claim-falsifier");
        let groups = vec![vec![duplicate.clone()], vec![duplicate]];
        let mut findings = Vec::new();
        assert_eq!(
            duplicate_paths(&groups, &mut findings),
            BTreeSet::from(["AGENT:claim-falsifier".to_owned()])
        );
        assert_eq!(findings[0].code, "duplicate_component_path");
    }

    #[test]
    fn expected_required_actual_pair_is_not_a_raw_path_conflict() {
        let mut required = entry("AGENT:claim-falsifier");
        required.active_status = ActiveStatus::Required;
        let groups = vec![vec![required], vec![entry("AGENT:claim-falsifier")]];
        let mut findings = Vec::new();
        assert!(duplicate_paths(&groups, &mut findings).is_empty());
        assert!(findings.is_empty());
    }

    #[test]
    fn duplicate_stable_id_conflicts_preserve_required_actual_pairing() {
        let mut required = entry("AGENT:claim-falsifier");
        required.active_status = ActiveStatus::Required;
        let actual = entry("AGENT:claim-falsifier");
        assert!(duplicate_stable_ids(&[vec![required, actual.clone()]]).is_empty());
        assert_eq!(
            duplicate_stable_ids(&[vec![actual.clone()], vec![actual]]),
            BTreeSet::from(["AGENT:claim-falsifier".to_owned()])
        );
    }
}
