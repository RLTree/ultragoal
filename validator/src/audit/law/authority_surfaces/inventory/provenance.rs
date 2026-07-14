use serde_json::Value;

pub(super) fn missing_generated_provenance(value: &Value) -> bool {
    if value
        .get("generated_from")
        .and_then(Value::as_str)
        .is_some()
        || value.pointer("/provenance/generated_from").is_some()
        || value.get("source_spec").is_some()
    {
        return false;
    }
    let provenance = value.get("provenance").unwrap_or(&Value::Null);
    !(text(provenance, "generated_artifact_type").is_some()
        && text(provenance, "validator_run_id").is_some()
        && text(provenance, "input_manifest_digest").is_some()
        && provenance.pointer("/validator_receipt/path").is_some()
        && provenance.pointer("/validator_receipt/digest").is_some())
}

pub(super) fn runtime_fixture_claims_artifact_truth(value: &Value) -> bool {
    value
        .pointer("/runtime_normalized_fixture/artifact_truth")
        .and_then(Value::as_bool)
        == Some(true)
        || value
            .get("runtime_normalized_fixture_artifact_truth")
            .and_then(Value::as_bool)
            == Some(true)
}

pub(super) fn hand_edits_allowed(value: &Value) -> bool {
    value.get("hand_edited").and_then(Value::as_bool) == Some(true)
        || value.get("manual_edit").and_then(Value::as_bool) == Some(true)
        || value.get("manual_edits_allowed").and_then(Value::as_bool) == Some(true)
}

pub(super) fn has_row_provenance(row: &Value) -> bool {
    let has_owner_surface =
        text(row, "current_owner_surface").is_some() || text(row, "owner_surface").is_some();
    let has_source_binding =
        row.get("source_spec").is_some() || text(row, "validator_check_id").is_some();
    has_owner_surface && has_source_binding
}

fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty())
}
