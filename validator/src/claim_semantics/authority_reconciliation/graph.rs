mod route;
mod scope;

use crate::audit::contract::Failure;
use crate::claim_semantics::{array_strings, str_field};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub(super) fn check(registry: &Value, root: &Path, out: &mut Vec<Failure>) {
    let graph_path = root.join(
        "docs/ultragoal-contract-2026-07-successor-v2/FINAL-CONTRACT/IMPLEMENTATION_DEPENDENCY_GRAPH.json",
    );
    let graph = match crate::json_boundary::read_json(&graph_path) {
        Ok(value) => value,
        Err(error) => {
            out.push(Failure::new(
                "authority-graph",
                "graph_ref_unavailable",
                error,
            ));
            return;
        }
    };
    compare_nodes(registry, &graph, out);
    route::check(registry, out);
    route::check_lane_contracts(registry, out);
    compare_ref(registry, &graph_path, out);
}

fn compare_nodes(registry: &Value, graph: &Value, out: &mut Vec<Failure>) {
    let expected = graph
        .get("nodes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let actual = registry
        .get("lanes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let scopes = graph
        .get("write_scopes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|scope| scope.get("scope_id").and_then(Value::as_str))
        .collect::<BTreeSet<_>>();
    let scope_owners = graph
        .get("write_scopes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|scope| {
            Some((
                scope.get("scope_id")?.as_str()?.to_owned(),
                scope.get("owner")?.as_str()?.to_owned(),
            ))
        })
        .collect::<Vec<_>>();
    let scope_owner = scope_owners.iter().cloned().collect::<BTreeMap<_, _>>();
    scope::check(registry, graph, &scopes, out);
    let root_only = registry
        .get("root_only_overrides")
        .and_then(Value::as_object)
        .map(|rows| rows.keys().cloned().collect::<BTreeSet<_>>())
        .unwrap_or_default();
    let expected_ids = (0..18)
        .map(|index| format!("N{index:02}"))
        .collect::<BTreeSet<_>>();
    let actual_ids = actual
        .iter()
        .map(|lane| str_field(lane, "id"))
        .collect::<BTreeSet<_>>();
    if actual_ids != expected_ids {
        out.push(Failure::new(
            "authority-graph",
            "lane_id_set_mismatch",
            "N00-N17",
        ));
    }
    let expected_by_id = expected
        .iter()
        .filter_map(|node| {
            let raw = str_field(node, "node_id");
            raw.strip_prefix('N')
                .and_then(|id| id.get(..2))
                .map(|id| (format!("N{id}"), node))
        })
        .collect::<BTreeMap<_, _>>();
    for lane in actual {
        let id = str_field(&lane, "id");
        let Some(node) = expected_by_id.get(&id) else {
            continue;
        };
        if str_field(&lane, "owner") != str_field(node, "owner") {
            out.push(Failure::new(
                "authority-graph",
                "lane_owner_mismatch",
                id.clone(),
            ));
        }
        let deps = array_strings(&lane, "dependencies");
        let expected_deps = node
            .get("depends_on")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|dep| dep.as_str().and_then(|value| value.get(..3)))
            .map(str::to_owned)
            .collect::<Vec<_>>();
        if deps != expected_deps {
            out.push(Failure::new(
                "authority-graph",
                "lane_dependency_mismatch",
                id.clone(),
            ));
        }
        let authority = str_field(&lane, "authority");
        let scopes_for_lane = array_strings(&lane, "scope_ids");
        if root_only.contains(&id) {
            if authority != "root_only" || !scopes_for_lane.is_empty() {
                out.push(Failure::new(
                    "authority-graph",
                    "root_only_scope_not_empty",
                    id.clone(),
                ));
            }
            let reason = registry
                .pointer(&format!("/root_only_overrides/{id}/reason"))
                .and_then(Value::as_str);
            if !matches!(
                reason,
                Some("no_compatible_write_scope" | "root_claim_authority")
            ) {
                out.push(Failure::new(
                    "authority-graph",
                    "root_only_override_reason_invalid",
                    id.clone(),
                ));
            }
            if reason == Some("no_compatible_write_scope")
                && scope_owners
                    .iter()
                    .any(|(_, owner)| owner.as_str() == str_field(&lane, "owner").as_str())
            {
                out.push(Failure::new(
                    "authority-graph",
                    "root_only_override_hides_compatible_scope",
                    id.clone(),
                ));
            }
        } else {
            if scopes_for_lane
                .iter()
                .any(|scope| !scopes.contains(scope.as_str()))
            {
                out.push(Failure::new(
                    "authority-graph",
                    "unknown_registry_scope",
                    id.clone(),
                ));
            }
            if authority == "lane_write"
                && scopes_for_lane.iter().any(|scope| {
                    scope_owner.get(scope).map(String::as_str)
                        != Some(str_field(&lane, "owner").as_str())
                })
            {
                out.push(Failure::new(
                    "authority-graph",
                    "lane_write_scope_owner_mismatch",
                    id.clone(),
                ));
            }
            if authority == "read_only"
                && scopes_for_lane.iter().any(|scope| {
                    scope_owner.get(scope).map(String::as_str)
                        == Some(str_field(&lane, "owner").as_str())
                })
            {
                out.push(Failure::new(
                    "authority-graph",
                    "read_only_scope_has_write_owner",
                    id,
                ));
            }
        }
    }
}

fn compare_ref(registry: &Value, path: &Path, out: &mut Vec<Failure>) {
    let Some(row) = registry.pointer("/source_context/refs/graph") else {
        out.push(Failure::new(
            "authority-graph",
            "current_ref_missing",
            "graph",
        ));
        return;
    };
    let actual = crate::digest::file(path).unwrap_or_default();
    if str_field(row, "validity") != "current_exact" || str_field(row, "digest") != actual {
        out.push(Failure::new(
            "authority-graph",
            "current_ref_digest_mismatch",
            "graph",
        ));
    }
}
