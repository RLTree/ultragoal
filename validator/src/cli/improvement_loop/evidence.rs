use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

const SCHEMA: &str = "harness-ultragoal.improvement-loop-stage-evidence.v1";
const AUTHORITY: &str = "cli_parsed_package_static_stage_evidence";
const CANDIDATE_BINDING: &str = "package_static_source_evidence";

pub(super) fn stage_evidence_failures(root: &Path, loop_item: &Value) -> Vec<String> {
    let mut out = Vec::new();
    let path = required_str(loop_item, "stage_evidence_path", &mut out);
    let expected_digest = required_str(loop_item, "stage_evidence_digest", &mut out);
    required_value(loop_item, "stage_evidence_schema", SCHEMA, &mut out);
    if path.is_empty() {
        return out;
    }
    if !is_safe_relative_path(&path) {
        out.push("improvement_loop_stage_evidence_path_invalid".to_string());
        return out;
    }
    let actual_digest = crate::digest::file(&root.join(&path)).unwrap_or_else(|_| {
        out.push("improvement_loop_stage_evidence_missing_file".to_string());
        crate::digest::ZERO.to_string()
    });
    if !expected_digest.is_empty() && actual_digest != expected_digest {
        out.push("improvement_loop_stage_evidence_digest_mismatch".to_string());
    }
    let doc = crate::json_boundary::read_json(&root.join(&path)).unwrap_or_else(|err| {
        out.push(format!(
            "improvement_loop_stage_evidence_json_missing_or_malformed:{err}"
        ));
        Value::Null
    });
    validate_doc(root, loop_item, &doc, &mut out);
    out
}

fn validate_doc(root: &Path, loop_item: &Value, doc: &Value, out: &mut Vec<String>) {
    required_value(doc, "schema", SCHEMA, out);
    required_value(doc, "law_id", super::LAW_ID, out);
    required_value(doc, "status", "pass", out);
    required_value(doc, "authority", AUTHORITY, out);
    required_value(doc, "candidate_binding", CANDIDATE_BINDING, out);
    let loop_id = loop_item
        .get("loop_id")
        .and_then(Value::as_str)
        .unwrap_or_default();
    required_value(doc, "loop_id", loop_id, out);
    let observed = stage_set(doc);
    for required in super::registry::REQUIRED_STAGES {
        if !observed.contains(required) {
            out.push(format!(
                "improvement_loop_stage_evidence_missing:{required}"
            ));
        }
    }
    for stage in doc
        .get("stages")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        validate_stage(root, stage, out);
    }
}

fn validate_stage(root: &Path, stage: &Value, out: &mut Vec<String>) {
    required_value(stage, "status", "pass", out);
    for field in [
        "source_paths",
        "receipt_paths",
        "command_ids",
        "forbidden_substitutions_rejected",
    ] {
        let values = string_array(stage.get(field));
        if values.is_empty() {
            let id = stage
                .get("stage_id")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            out.push(format!(
                "improvement_loop_stage_evidence_empty_field:{id}:{field}"
            ));
        }
        if field.ends_with("_paths") {
            for rel in values {
                if !is_safe_relative_path(&rel) || !root.join(&rel).exists() {
                    out.push(format!("improvement_loop_stage_evidence_bad_path:{rel}"));
                }
            }
        }
    }
}

fn stage_set(doc: &Value) -> BTreeSet<&str> {
    doc.get("stages")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|stage| {
            let id = stage.get("stage_id").and_then(Value::as_str)?;
            let status = stage.get("status").and_then(Value::as_str)?;
            (status == "pass").then_some(id)
        })
        .collect()
}

fn required_str(value: &Value, key: &str, out: &mut Vec<String>) -> String {
    match value.get(key).and_then(Value::as_str) {
        Some(value) if !value.is_empty() => value.to_string(),
        _ => {
            out.push(format!(
                "improvement_loop_stage_evidence_missing_field:{key}"
            ));
            String::new()
        }
    }
}

fn required_value(value: &Value, key: &str, expected: &str, out: &mut Vec<String>) {
    if value.get(key).and_then(Value::as_str) != Some(expected) {
        out.push(format!(
            "improvement_loop_stage_evidence_field_mismatch:{key}"
        ));
    }
}

fn string_array(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

fn is_safe_relative_path(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('/')
        && !value
            .split('/')
            .any(|component| component.is_empty() || component == "." || component == "..")
}
