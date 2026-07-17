use crate::audit::contract::Failure;
use crate::claim_semantics::str_field;
use serde_json::Value;
use std::collections::BTreeSet;

pub(super) fn check(
    registry: &Value,
    graph: &Value,
    expected_scopes: &BTreeSet<&str>,
    out: &mut Vec<Failure>,
) {
    let mappings = registry
        .get("scope_mappings")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let actual = mappings
        .iter()
        .map(|mapping| str_field(mapping, "scope_id"))
        .collect::<BTreeSet<_>>();
    let expected = expected_scopes.iter().copied().collect::<BTreeSet<_>>();
    if actual.iter().map(String::as_str).collect::<BTreeSet<_>>() != expected
        || actual.len() != mappings.len()
    {
        out.push(Failure::new(
            "authority-graph",
            "scope_mapping_set_mismatch",
            "scope_mappings",
        ));
        return;
    }
    for scope in graph["write_scopes"].as_array().into_iter().flatten() {
        let id = str_field(scope, "scope_id");
        let Some(mapping) = mappings
            .iter()
            .find(|mapping| str_field(mapping, "scope_id") == id)
        else {
            continue;
        };
        if set(scope, "exclusive_paths") != set(mapping, "contract_roots") {
            out.push(Failure::new(
                "authority-graph",
                "scope_contract_root_mismatch",
                id.clone(),
            ));
        }
        let expected_current = current_roots(&id, &set(scope, "exclusive_paths"));
        if set(mapping, "owned_roots") != expected_current
            || set(mapping, "generated_roots") != set(scope, "generated_outputs")
        {
            out.push(Failure::new(
                "authority-graph",
                "scope_current_root_mismatch",
                id,
            ));
        }
    }
}

fn current_roots(id: &str, contract_roots: &BTreeSet<String>) -> BTreeSet<String> {
    match id {
        "WS-FIT" => set_from([
            "validator/src/repository_fit/",
            "templates/repository-fit/",
            "tests/fit/",
        ]),
        "WS-ROUTINE" => set_from([
            "validator/src/routine_work/",
            "validator/src/fixture_scheduler/",
            "tests/routine/",
        ]),
        "WS-OBSERVE" => set_from([
            "validator/src/observability/",
            "schemas/observability/",
            "tests/observability/",
        ]),
        "WS-EVAL" => set_from([
            "validator/src/evaluation/",
            "validator/src/fixture_scheduler/",
            "evals/",
            "tests/eval/",
        ]),
        "WS-MIGRATION" => set_from(["validator/src/migration/", "migration/", "tests/migration/"]),
        _ => contract_roots.clone(),
    }
}

fn set(row: &Value, key: &str) -> BTreeSet<String> {
    row.get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}

fn set_from<const N: usize>(items: [&str; N]) -> BTreeSet<String> {
    items.into_iter().map(str::to_owned).collect()
}
