use super::routing::RoutingData;
use super::types::{ActiveStatus, AuthorityState, InventoryEntry, InventoryFinding};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

fn merge_one(
    entries: &mut BTreeMap<String, InventoryEntry>,
    mut incoming: InventoryEntry,
    findings: &mut Vec<InventoryFinding>,
) {
    incoming.normalize();
    let Some(existing) = entries.get(&incoming.stable_id).cloned() else {
        entries.insert(incoming.stable_id.clone(), incoming);
        return;
    };
    let existing_required = matches!(
        existing.active_status,
        ActiveStatus::Required | ActiveStatus::Missing
    );
    let incoming_required = matches!(
        incoming.active_status,
        ActiveStatus::Required | ActiveStatus::Missing
    );
    if existing_required ^ incoming_required {
        let (expected, mut actual) = if existing_required {
            (existing, incoming)
        } else {
            (incoming, existing)
        };
        if !expected.relative_path.starts_with("@semantic/")
            && expected.relative_path != actual.relative_path
        {
            findings.push(InventoryFinding::error(
                "renamed_required_component",
                Some(&actual.stable_id),
                Some(&actual.relative_path),
                format!(
                    "expected path {}, found {}",
                    expected.relative_path, actual.relative_path
                ),
            ));
        }
        actual.owner_role = expected.owner_role;
        actual.authority_state = expected.authority_state;
        actual.references.extend(expected.references);
        actual
            .input_provenance
            .push(format!("required-path:{}", expected.relative_path));
        actual.normalize();
        entries.insert(actual.stable_id.clone(), actual);
        return;
    }
    findings.push(InventoryFinding::error(
        "duplicate_stable_id",
        Some(&incoming.stable_id),
        Some(&incoming.relative_path),
        format!("also defined at {}", existing.relative_path),
    ));
}

fn missing_required(
    entries: &mut BTreeMap<String, InventoryEntry>,
    findings: &mut Vec<InventoryFinding>,
) {
    for entry in entries.values_mut() {
        if entry.active_status == ActiveStatus::Required {
            entry.active_status = ActiveStatus::Missing;
            findings.push(InventoryFinding::error(
                "missing_required_component",
                Some(&entry.stable_id),
                Some(&entry.relative_path),
                "required contract component was not discovered".to_owned(),
            ));
        }
    }
}

fn duplicate_paths(
    entries: &BTreeMap<String, InventoryEntry>,
    findings: &mut Vec<InventoryFinding>,
) {
    let mut paths: BTreeMap<&str, &str> = BTreeMap::new();
    for entry in entries.values() {
        if entry.relative_path.contains("#/") || entry.kind == "source-symbol-implementation" {
            continue;
        }
        if let Some(prior) = paths.insert(&entry.relative_path, &entry.stable_id) {
            findings.push(InventoryFinding::error(
                "duplicate_component_path",
                Some(&entry.stable_id),
                Some(&entry.relative_path),
                format!("path is also owned by {prior}"),
            ));
        }
    }
}

fn schema_reference_exists(root: &Path, entry: &InventoryEntry, reference: &str) -> bool {
    if reference.starts_with('#') || reference.contains("://") {
        return true;
    }
    let path_part = reference.split('#').next().unwrap_or_default();
    if path_part.is_empty() {
        return true;
    }
    let parent = Path::new(&entry.relative_path)
        .parent()
        .unwrap_or_else(|| Path::new(""));
    root.join(parent)
        .join(path_part)
        .canonicalize()
        .is_ok_and(|resolved| resolved.starts_with(root) && resolved.is_file())
}

fn unresolved_references(
    root: &Path,
    entries: &BTreeMap<String, InventoryEntry>,
    findings: &mut Vec<InventoryFinding>,
) {
    let ids = entries.keys().map(String::as_str).collect::<BTreeSet<_>>();
    for entry in entries.values() {
        if entry.kind == "generated-surface"
            && let Some(generator) = entry.generator.as_deref()
            && !ids.contains(generator)
        {
            findings.push(InventoryFinding::error(
                "unresolved_generator",
                Some(&entry.stable_id),
                Some(&entry.relative_path),
                "generated surface names an unknown Harness tool".to_owned(),
            ));
        }
        for reference in &entry.references {
            let semantic = [
                "PS-",
                "HCT-",
                "CL-",
                "REQ-",
                "SKILL:",
                "AGENT:",
                "COMMAND:",
                "CONTRACT-REGISTRY:",
                "API:",
                "J-",
                "N0",
                "N1",
                "WS-",
                "SRC-",
                "TRUTH-LAYER:",
            ]
            .iter()
            .any(|prefix| reference.starts_with(prefix));
            let resolved = if semantic {
                ids.contains(reference.as_str())
            } else if entry.kind == "schema" {
                schema_reference_exists(root, entry, reference)
            } else {
                true
            };
            if !resolved {
                findings.push(InventoryFinding::error(
                    "unresolved_reference",
                    Some(&entry.stable_id),
                    Some(&entry.relative_path),
                    format!("unresolved reference {reference}"),
                ));
            }
        }
    }
}

fn parallel_authority(
    entries: &BTreeMap<String, InventoryEntry>,
    findings: &mut Vec<InventoryFinding>,
) {
    for legacy in entries.values().filter(|entry| {
        entry.authority_state == AuthorityState::Legacy
            && entry.active_status == ActiveStatus::Active
    }) {
        findings.push(InventoryFinding::error(
            "parallel_authority",
            Some(&legacy.stable_id),
            Some(&legacy.relative_path),
            "legacy surface remains authoritative without an adopted disposition".to_owned(),
        ));
    }
    let plugin = entries.get("PLUGIN-MANIFEST");
    let draft = entries
        .values()
        .find(|entry| entry.relative_path == "plugin-manifest-draft.json");
    if let (Some(plugin), Some(draft)) = (plugin, draft)
        && plugin.digest_sha256 != draft.digest_sha256
    {
        findings.push(InventoryFinding::error(
            "projection_drift",
            Some(&plugin.stable_id),
            Some(&plugin.relative_path),
            "live plugin projection differs from legacy manifest draft".to_owned(),
        ));
    }
}

pub(crate) fn reconcile(
    root: &Path,
    groups: Vec<Vec<InventoryEntry>>,
    mut findings: Vec<InventoryFinding>,
    routing: &RoutingData,
) -> (Vec<InventoryEntry>, Vec<InventoryFinding>) {
    let mut entries = BTreeMap::new();
    for group in groups {
        for entry in group {
            merge_one(&mut entries, entry, &mut findings);
        }
    }
    routing.apply(&mut entries, &mut findings);
    missing_required(&mut entries, &mut findings);
    duplicate_paths(&entries, &mut findings);
    unresolved_references(root, &entries, &mut findings);
    parallel_authority(&entries, &mut findings);
    let mut entries = entries.into_values().collect::<Vec<_>>();
    entries.sort_by(|left, right| {
        left.stable_id
            .cmp(&right.stable_id)
            .then(left.relative_path.cmp(&right.relative_path))
    });
    findings.sort();
    findings.dedup();
    (entries, findings)
}
