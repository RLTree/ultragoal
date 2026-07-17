use crate::audit::contract::Failure;
use crate::claim_semantics::str_field;
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

pub(super) fn check(registry: &Value, root: &Path, out: &mut Vec<Failure>) {
    let claim_path = root
        .join("docs/ultragoal-contract-2026-07-successor-v2/FINAL-CONTRACT/CLAIM_REGISTRY.json");
    let claim_registry = match crate::json_boundary::read_json(&claim_path) {
        Ok(value) => value,
        Err(error) => {
            out.push(Failure::new(
                "authority-claims",
                "claim_registry_unavailable",
                error,
            ));
            return;
        }
    };
    let completion = match crate::json_boundary::read_json(&root.join("COMPLETION_MANIFEST.json")) {
        Ok(value) => value,
        Err(error) => {
            out.push(Failure::new(
                "authority-claims",
                "completion_manifest_unavailable",
                error,
            ));
            return;
        }
    };
    let backlog = match crate::json_boundary::read_json(&root.join("VERIFICATION_BACKLOG.json")) {
        Ok(value) => value,
        Err(error) => {
            out.push(Failure::new(
                "authority-claims",
                "verification_backlog_unavailable",
                error,
            ));
            return;
        }
    };
    compare_ids(&claim_registry, &completion, &backlog, out);
    compare_projection_digest(registry, &completion, &backlog, out);
    compare_ref(registry, &claim_path, out);
    super::freeze::payload_refs(registry, root, out);
}

fn compare_ids(
    claim_registry: &Value,
    completion: &Value,
    backlog: &Value,
    out: &mut Vec<Failure>,
) {
    let registry_rows = claim_registry
        .pointer("/claims")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let registry_ids = registry_rows
        .iter()
        .filter_map(|row| {
            row.get("claim_id")
                .or_else(|| row.get("id"))
                .and_then(Value::as_str)
        })
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let required = completion
        .get("required_claim_ids")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let claims = completion
        .get("claims")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let backlog_rows = backlog
        .get("rows")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let backlog_ids = backlog_rows
        .iter()
        .filter_map(|row| row.get("claim_id").and_then(Value::as_str))
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let completion_ids = claims
        .iter()
        .filter_map(|row| row.get("id").and_then(Value::as_str))
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if duplicate_or_different(&required)
        || duplicate_or_different(&registry_ids)
        || duplicate_or_different(&backlog_ids)
        || duplicate_or_different(&completion_ids)
    {
        out.push(Failure::new(
            "authority-claims",
            "claim_id_duplicate",
            "claims/backlog",
        ));
    }
    let topological = claim_registry
        .get("claim_topological_order")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let required_hash = crate::digest::bytes(
        serde_json::to_string(&required)
            .unwrap_or_default()
            .as_bytes(),
    );
    if required != completion_ids
        || required != registry_ids
        || required != backlog_ids
        || str_field(completion, "required_claim_ids_hash") != required_hash
    {
        out.push(Failure::new(
            "authority-claims",
            "claim_registry_projection_mismatch",
            "required_claim_ids",
        ));
    }
    if duplicate_or_different(&topological)
        || topological.iter().collect::<BTreeSet<_>>() != required.iter().collect::<BTreeSet<_>>()
    {
        out.push(Failure::new(
            "authority-claims",
            "claim_topological_order_mismatch",
            "CLAIM_REGISTRY.json",
        ));
    }
    for claim in &claims {
        let id = str_field(claim, "id");
        let expected_row = format!(
            "BACKLOG-{:03}",
            required
                .iter()
                .position(|value| value == &id)
                .map(|index| index + 1)
                .unwrap_or_default()
        );
        let row_id = str_field(claim, "backlog_row_id");
        let row_claim = backlog_rows
            .iter()
            .find(|row| str_field(row, "id") == row_id)
            .map(|row| str_field(row, "claim_id"))
            .unwrap_or_default();
        if row_id != expected_row || row_claim != id {
            out.push(Failure::new(
                "authority-claims",
                "backlog_claim_join_mismatch",
                id.clone(),
            ));
        }
        if let Some(source) = registry_rows
            .iter()
            .find(|row| str_field(row, "claim_id") == id)
            && str_field(claim, "title") != str_field(source, "name")
        {
            out.push(Failure::new("authority-claims", "claim_name_mismatch", id));
        }
    }
    let owners = registry_rows
        .iter()
        .filter_map(|row| row.get("claim_decision_owner").and_then(Value::as_str))
        .collect::<BTreeSet<_>>();
    if owners.is_empty() {
        out.push(Failure::new(
            "authority-claims",
            "claim_owner_missing",
            "CLAIM_REGISTRY.json",
        ));
    }
}

fn duplicate_or_different(values: &[String]) -> bool {
    values.iter().collect::<BTreeSet<_>>().len() != values.len()
}

fn compare_projection_digest(
    registry: &Value,
    completion: &Value,
    backlog: &Value,
    out: &mut Vec<Failure>,
) {
    let expected = registry_preimage(registry);
    if str_field(completion, "verification_backlog_digest") != expected
        || str_field(backlog, "manifest_digest") != expected
        || str_field(completion, "registry_preimage_digest") != expected
        || str_field(backlog, "registry_preimage_digest") != expected
    {
        out.push(Failure::new(
            "authority-claims",
            "projection_registry_preimage_mismatch",
            expected,
        ));
    }
}

pub(crate) fn registry_preimage(registry: &Value) -> String {
    let mut value = registry.clone();
    if let Some(object) = value.as_object_mut() {
        if let Some(freeze) = object.get_mut("root_freeze").and_then(Value::as_object_mut) {
            freeze.remove("payload_refs");
        }
    }
    crate::digest::canonical_json(&value)
}

fn compare_ref(registry: &Value, path: &Path, out: &mut Vec<Failure>) {
    let Some(row) = registry.pointer("/source_context/refs/claims") else {
        out.push(Failure::new(
            "authority-claims",
            "current_ref_missing",
            "claims",
        ));
        return;
    };
    if str_field(row, "validity") != "current_exact"
        || str_field(row, "digest") != crate::digest::file(path).unwrap_or_default()
    {
        out.push(Failure::new(
            "authority-claims",
            "current_ref_digest_mismatch",
            "claims",
        ));
    }
}
