use super::{RETAINED_KIND, RouteSpec, STRUCTURAL_WITNESS_KIND, by_legacy_name, by_route_id};
use crate::context::ReadSession;
use crate::inventory::fs::read_bounded;
use crate::inventory::types::{InventoryEntry, InventoryError, InventoryFinding};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

const MAX_WRAPPER_BYTES: u64 = 64 * 1024;

fn expected_skill(route: &RouteSpec) -> String {
    let legacy = route.legacy_name;
    let canonical = route.canonical_name;
    format!(
        "---\nname: {legacy}\ndescription: Deprecated compatibility alias for explicit `$harness-ultragoal:{legacy}` requests. Preserve the request and route it to `$harness-ultragoal:{canonical}`; do not use this alias as independent workflow authority.\n---\n\n# Deprecated Compatibility Route\n\n> Compatibility warning: this legacy alias is not an independent workflow or authority. Its canonical target is `$harness-ultragoal:{canonical}`.\n\n1. Preserve the user's full request, context, constraints, and authorized effects unchanged.\n2. Before routing, inspect the request and supplied context for Harness Ultragoal skill tokens. If it contains another distinct explicit Harness Ultragoal skill token or selects multiple compatibility routes, report a causal compatibility-route conflict and perform no routing or effect.\n3. Invoke `$harness-ultragoal:{canonical}` with that preserved input.\n4. Follow only the canonical target's current contract. Do not restore or apply legacy lane, gate, receipt, finalizer, command, tool, helper, schema, state-store, or generated authority from this wrapper.\n5. Perform no hidden writes or external effects while resolving the route. Any later effect must remain authorized by the original request and the canonical target.\n6. Fail closed if the canonical target is unavailable: report the exact blocker and do not fall back, infer semantic equivalence, or claim adoption, discovery, runtime behavior, retirement, readiness, release, or completion.\n7. Never substitute documentation, tests, receipts, generated rows, telemetry, signatures, or provenance for the requested product behavior.\n"
    )
}

fn expected_metadata(route: &RouteSpec) -> String {
    let legacy = route.legacy_name;
    let display = route.display_name;
    format!(
        "interface:\n  display_name: \"{display} (Legacy Alias)\"\n  short_description: \"Route this legacy alias to its canonical skill\"\n  default_prompt: \"Use $harness-ultragoal:{legacy} to preserve and route my unchanged request through its canonical skill.\"\npolicy:\n  allow_implicit_invocation: false\n"
    )
}

fn regular_non_symlink(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .is_ok_and(|metadata| metadata.is_file() && !metadata.file_type().is_symlink())
}

pub(crate) fn inspect_wrapper(
    reads: &ReadSession,
    root: &Path,
    skill_path: &Path,
    directory_name: &str,
    declared_name: &str,
) -> Result<Option<&'static RouteSpec>, InventoryError> {
    let Some(route) = by_legacy_name(declared_name) else {
        return Ok(None);
    };
    let metadata_path = root.join(route.metadata_path());
    if directory_name != route.legacy_name
        || skill_path != root.join(route.skill_path())
        || !regular_non_symlink(skill_path)
        || !regular_non_symlink(&metadata_path)
    {
        return Ok(None);
    }
    let skill = read_bounded(reads, skill_path, MAX_WRAPPER_BYTES)?;
    let metadata = read_bounded(reads, &metadata_path, MAX_WRAPPER_BYTES)?;
    Ok((skill == expected_skill(route).as_bytes()
        && metadata == expected_metadata(route).as_bytes())
    .then_some(route))
}

fn entry_has_structural_witness(entry: &InventoryEntry, route: &RouteSpec) -> bool {
    entry.kind == STRUCTURAL_WITNESS_KIND
        && entry.stable_id == route.stable_id()
        && entry.relative_path == route.skill_path()
        && entry.references == [route.canonical_id()]
        && entry.input_provenance == [route.metadata_path()]
}

pub(crate) fn registry_route_is_compiled(
    route_id: &str,
    exact_stable_id: Option<&str>,
    canonical_target: &str,
    proof_refs: &[String],
) -> bool {
    by_route_id(route_id).is_some_and(|route| {
        exact_stable_id == Some(route.stable_id().as_str())
            && canonical_target == route.canonical_id()
            && route.proof_refs_match(proof_refs)
    })
}

pub(crate) fn retention_candidates(
    entries: &BTreeMap<String, InventoryEntry>,
    findings: &[InventoryFinding],
) -> BTreeSet<String> {
    let invalid = findings
        .iter()
        .filter(|finding| {
            matches!(
                finding.code.as_str(),
                "duplicate_stable_id" | "renamed_required_component" | "duplicate_component_path"
            )
        })
        .filter_map(|finding| finding.entry_id.as_deref())
        .collect::<BTreeSet<_>>();
    entries
        .values()
        .filter(|entry| {
            entry.kind == "skill"
                && entry.authority_state == crate::inventory::types::AuthorityState::Canonical
                && entry.active_status == crate::inventory::types::ActiveStatus::Active
                && !invalid.contains(entry.stable_id.as_str())
        })
        .map(|entry| entry.stable_id.clone())
        .collect()
}

pub(crate) fn apply_route(
    entry: &mut InventoryEntry,
    route_id: &str,
    exact_stable_id: Option<&str>,
    canonical_target: &str,
    requests_retention: bool,
    target_present: bool,
    findings: &mut Vec<InventoryFinding>,
) {
    let compiled = by_route_id(route_id).is_some_and(|route| {
        exact_stable_id == Some(route.stable_id().as_str())
            && canonical_target == route.canonical_id()
            && entry_has_structural_witness(entry, route)
    });
    if entry.kind == STRUCTURAL_WITNESS_KIND && !compiled {
        findings.push(InventoryFinding::error(
            "invalid_compatibility_route_transition",
            Some(&entry.stable_id),
            Some(&entry.relative_path),
            "compatibility wrapper is not bound to its exact compiled route".to_owned(),
        ));
    }
    if !requests_retention {
        return;
    }
    if compiled && target_present {
        entry.kind = RETAINED_KIND.to_owned();
        findings.push(InventoryFinding::warning(
            "compatibility_route_retained",
            Some(&entry.stable_id),
            Some(&entry.relative_path),
            "explicit-only compatibility route is preserved; retirement, equivalence, readiness, release, and completion remain unproven".to_owned(),
        ));
    } else {
        findings.push(InventoryFinding::error(
            "invalid_compatibility_route_transition",
            Some(&entry.stable_id),
            Some(&entry.relative_path),
            "retained compatibility route lacks its wrapper or canonical candidate".to_owned(),
        ));
    }
}
