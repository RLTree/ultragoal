use super::overlap;
use crate::audit::contract::Failure;
use crate::claim_semantics::str_field;
use serde_json::Value;
use std::collections::BTreeSet;

const NON_P0_GATES: [&str; 3] = ["compile", "namespace", "standards"];
const MATERIAL_GATES: [&str; 4] = ["product", "signoff", "release", "completion"];

pub(super) fn check_gate_config(registry: &Value, out: &mut Vec<Failure>) {
    let Some(gates) = registry.get("prelaunch_gates").and_then(Value::as_array) else {
        out.push(Failure::new(
            "authority-lease",
            "lease_gate_config_missing",
            "prelaunch_gates",
        ));
        return;
    };
    for gate in gates {
        let id = str_field(gate, "id");
        let actual = array(gate, "applies_to");
        let expected = match id.as_str() {
            "compile" | "namespace" | "standards" => vec!["non_p0_lane_issuance"],
            "four_persona_exposure" => MATERIAL_GATES.to_vec(),
            _ => {
                out.push(Failure::new("authority-lease", "lease_gate_unknown", id));
                continue;
            }
        };
        if actual != expected {
            out.push(Failure::new(
                "authority-lease",
                "lease_gate_applicability_mismatch",
                id,
            ));
        }
    }
}

pub(super) fn check(record: &Value, registry: &Value, out: &mut Vec<Failure>) {
    if str_field(record, "status") == "unissued" {
        return;
    }
    if record.get("exception_id").and_then(Value::as_str) == Some("P0-DEBT-REPAIR") {
        return;
    }
    let lane_id = str_field(record, "lane_id");
    let Some(lane) = registry["lanes"]
        .as_array()
        .and_then(|lanes| lanes.iter().find(|lane| str_field(lane, "id") == lane_id))
    else {
        return;
    };
    check_epoch_and_state(lane, registry, out);
    check_required_gates(registry, out);
    check_dependencies(record, lane, registry, out);
    check_owned_scope(record, lane, registry, out);
}

fn check_epoch_and_state(lane: &Value, registry: &Value, out: &mut Vec<Failure>) {
    let adopted = registry
        .pointer("/pre_adoption_source/adoption_status")
        .and_then(Value::as_str)
        == Some("adopted_current_epoch");
    let n00_integrated = registry["lanes"]
        .as_array()
        .and_then(|lanes| lanes.iter().find(|lane| str_field(lane, "id") == "N00"))
        .is_some_and(|lane| {
            str_field(lane, "state") == "integrated" && lane["current_identity"].is_object()
        });
    if !adopted || !n00_integrated {
        out.push(Failure::new(
            "authority-lease",
            "lease_epoch_not_adopted",
            "N00/adoption_status",
        ));
    }
    let lane_id = str_field(lane, "id");
    let selectable = registry
        .pointer("/pre_adoption_source/eligible_scheduler_nodes")
        .and_then(Value::as_array)
        .is_some_and(|nodes| {
            nodes
                .iter()
                .any(|node| node.as_str() == Some(lane_id.as_str()))
        });
    if !selectable {
        out.push(Failure::new(
            "authority-lease",
            "lease_lane_not_current_epoch_eligible",
            lane_id,
        ));
    }
    if !matches!(
        str_field(lane, "state").as_str(),
        "ready" | "leased" | "candidate" | "under_review" | "rework" | "accepted" | "integrating"
    ) {
        out.push(Failure::new(
            "authority-lease",
            "lease_lane_state_not_issuable",
            str_field(lane, "id"),
        ));
    }
}

fn check_required_gates(registry: &Value, out: &mut Vec<Failure>) {
    let gates = registry
        .get("prelaunch_gates")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for id in NON_P0_GATES {
        let current = gates
            .iter()
            .find(|gate| str_field(gate, "id") == id)
            .is_some_and(|gate| {
                str_field(gate, "status") == "current"
                    && str_field(gate, "evidence_status") == "current"
            });
        if !current {
            out.push(Failure::new(
                "authority-lease",
                "lease_prelaunch_gate_blocked",
                id,
            ));
        }
    }
}

fn check_dependencies(record: &Value, lane: &Value, registry: &Value, out: &mut Vec<Failure>) {
    let identities = record
        .pointer("/consumed_set/dependency_identities")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for dependency in lane
        .pointer("/consumption_contract/dependency_ids")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
    {
        let current = registry["lanes"].as_array().and_then(|lanes| {
            lanes
                .iter()
                .find(|lane| str_field(lane, "id") == dependency)
        });
        let exact = current.is_some_and(|current| {
            str_field(current, "state") == "integrated"
                && current["current_identity"].is_object()
                && identities
                    .iter()
                    .any(|identity| identity == &current["current_identity"])
        });
        if !exact {
            out.push(Failure::new(
                "authority-lease",
                "lease_dependency_not_current_integrated",
                dependency,
            ));
        }
    }
}

fn check_owned_scope(record: &Value, lane: &Value, registry: &Value, out: &mut Vec<Failure>) {
    let scopes = overlap::array_set(lane, "scope_ids");
    let mappings = registry
        .get("scope_mappings")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let declared = scopes
        .iter()
        .filter_map(|scope| {
            mappings
                .iter()
                .find(|mapping| str_field(mapping, "scope_id") == *scope)
        })
        .collect::<Vec<_>>();
    if declared.len() != scopes.len() {
        out.push(Failure::new(
            "authority-lease",
            "lease_scope_mapping_missing",
            str_field(lane, "id"),
        ));
        return;
    }
    for (surface, roots_key) in [
        ("owned_files", "owned_roots"),
        ("generated_outputs", "generated_roots"),
        ("fixtures", "fixture_roots"),
    ] {
        let roots = declared
            .iter()
            .flat_map(|mapping| array(mapping, roots_key))
            .collect::<BTreeSet<_>>();
        for path in overlap::array_set(record, surface) {
            if !roots.iter().any(|root| path_in_root(&path, root)) {
                out.push(Failure::new(
                    "authority-lease",
                    "lease_owned_surface_outside_scope",
                    format!("{surface}:{path}"),
                ));
            }
        }
    }
}

fn array(row: &Value, key: &str) -> Vec<String> {
    row.get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}

fn path_in_root(path: &str, root: &str) -> bool {
    let root = root.trim_end_matches('/');
    path == root || path.starts_with(&format!("{root}/"))
}
