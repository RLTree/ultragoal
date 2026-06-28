use crate::{json_boundary, schema_catalog};
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

mod requirements;
use requirements::{
    REQUIRED_CLASSES, REQUIRED_FINDING_ARRAYS, REQUIRED_FINDING_STRINGS, REQUIRED_SOURCE_IDS,
};

const RECEIPT: &str = "validation_artifacts/harness/session-log-hardening-receipt.json";
const SCHEMA: &str = "session-log-hardening-receipt.schema.json";

pub fn package_failures(root: &Path, store: &schema_catalog::SchemaStore) -> Vec<String> {
    let mut out = Vec::new();
    let receipt = match json_boundary::read_json(&root.join(RECEIPT)) {
        Ok(value) => value,
        Err(err) => {
            out.push(format!("session_log_hardening_receipt_missing:{err}"));
            return out;
        }
    };

    out.extend(
        schema_catalog::schema_errors(store, SCHEMA, &receipt)
            .into_iter()
            .map(|err| format!("session_log_hardening_schema:{err}")),
    );
    match current_manifest_version(root) {
        Ok(version) if string(&receipt, "candidate_version") != version => {
            out.push("session_log_hardening_candidate_version_mismatch".to_string());
        }
        Err(err) => out.push(format!("session_log_hardening_version_unreadable:{err}")),
        _ => {}
    }
    match crate::package::inventory::package_digest(root) {
        Ok(digest) if string(&receipt, "package_digest") != digest => {
            out.push("session_log_hardening_package_digest_mismatch".to_string());
        }
        Err(err) => out.push(format!(
            "session_log_hardening_package_digest_unreadable:{err}"
        )),
        _ => {}
    }
    out.extend(source_failures(&receipt));
    out.extend(issue_class_failures(&receipt));
    out.extend(finding_failures(&receipt));
    out.extend(packet_failures(root, &receipt));
    out
}

fn source_failures(receipt: &Value) -> Vec<String> {
    let mut out = Vec::new();
    let sources = array(receipt, "audit_sources");
    let kinds = sources
        .iter()
        .filter_map(|row| row.get("source_kind").and_then(Value::as_str))
        .collect::<BTreeSet<_>>();
    for required in ["chronicle_summary", "session_log", "packet_summary"] {
        if !kinds.contains(required) {
            out.push(format!(
                "session_log_hardening_source_kind_missing:{required}"
            ));
        }
    }
    let ids = sources
        .iter()
        .filter_map(|row| row.get("source_id").and_then(Value::as_str))
        .collect::<BTreeSet<_>>();
    for required in REQUIRED_SOURCE_IDS {
        if !ids.contains(required) {
            out.push(format!(
                "session_log_hardening_source_id_missing:{required}"
            ));
        }
    }
    out
}

fn issue_class_failures(receipt: &Value) -> Vec<String> {
    let mut out = Vec::new();
    let classes = array(receipt, "required_issue_classes");
    let present = classes
        .iter()
        .filter_map(|row| row.get("class_id").and_then(Value::as_str))
        .collect::<BTreeSet<_>>();
    for required in REQUIRED_CLASSES {
        if !present.contains(required) {
            out.push(format!(
                "session_log_hardening_issue_class_missing:{required}"
            ));
        }
    }
    out
}

fn finding_failures(receipt: &Value) -> Vec<String> {
    let mut out = Vec::new();
    let findings = array(receipt, "findings");
    let source_ids = array(receipt, "audit_sources")
        .iter()
        .filter_map(|row| row.get("source_id").and_then(Value::as_str))
        .collect::<BTreeSet<_>>();
    let mut finding_ids = BTreeSet::new();
    let mut has_fixed_with_validator = false;
    for finding in findings {
        let id = string(finding, "finding_id");
        if !finding_ids.insert(id) {
            out.push(format!("session_log_hardening_duplicate_finding_id:{id}"));
        }
        let status = finding
            .get("enforcement_status")
            .and_then(Value::as_str)
            .unwrap_or("");
        if status != string(finding, "implementation_status") {
            out.push(format!(
                "session_log_hardening_implementation_status_mismatch:{id}"
            ));
        }
        for field in REQUIRED_FINDING_STRINGS {
            if string(finding, field).is_empty() {
                out.push(format!(
                    "session_log_hardening_finding_field_missing:{id}:{field}"
                ));
            }
        }
        for field in REQUIRED_FINDING_ARRAYS {
            if array(finding, field).is_empty() {
                out.push(format!(
                    "session_log_hardening_finding_array_missing:{id}:{field}"
                ));
            }
        }
        for source_ref in array(finding, "source_refs")
            .iter()
            .filter_map(|row| row.as_str())
        {
            if !source_ids.contains(source_ref) {
                out.push(format!(
                    "session_log_hardening_source_ref_unknown:{id}:{source_ref}"
                ));
            }
        }
        let artifact_types = array(finding, "artifact_types")
            .into_iter()
            .filter_map(Value::as_str)
            .collect::<BTreeSet<_>>();
        if status == "fixed"
            && (artifact_types.contains("validator")
                || artifact_types.contains("schema")
                || artifact_types.contains("receipt"))
        {
            has_fixed_with_validator = true;
        }
    }
    if !has_fixed_with_validator {
        out.push("session_log_hardening_no_deterministic_fix".to_string());
    }
    out.extend(issue_finding_link_failures(receipt, &finding_ids));
    out
}

fn issue_finding_link_failures(receipt: &Value, finding_ids: &BTreeSet<&str>) -> Vec<String> {
    let mut out = Vec::new();
    for issue in array(receipt, "required_issue_classes") {
        let class_id = string(issue, "class_id");
        for finding_id in array(issue, "finding_ids")
            .iter()
            .filter_map(|row| row.as_str())
        {
            if !finding_ids.contains(finding_id) {
                out.push(format!(
                    "session_log_hardening_issue_finding_unknown:{class_id}:{finding_id}"
                ));
            }
        }
    }
    out
}

fn packet_failures(root: &Path, receipt: &Value) -> Vec<String> {
    let mut out = Vec::new();
    let packet_rel = receipt
        .pointer("/packet_successor/path")
        .and_then(Value::as_str)
        .unwrap_or("");
    if packet_rel.is_empty() || !root.join(packet_rel).is_file() {
        out.push(format!("session_log_hardening_packet_missing:{packet_rel}"));
    }
    let ceiling = string(receipt, "current_claim_ceiling").to_ascii_lowercase();
    for unsupported in [
        "does not support live app registry",
        "plugins ui",
        "marketplace",
        "current reviewer exposure",
        "material review sign-off",
        "real user product fitness",
        "does not support 100% coverage",
    ] {
        if !ceiling.contains(unsupported) {
            out.push(format!(
                "session_log_hardening_claim_ceiling_missing:{unsupported}"
            ));
        }
    }
    out
}

fn array<'a>(value: &'a Value, key: &str) -> Vec<&'a Value> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|rows| rows.iter().collect())
        .unwrap_or_default()
}

fn string<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or("")
}

fn current_manifest_version(root: &Path) -> Result<String, String> {
    let manifest = json_boundary::read_json(&root.join("plugin-manifest-draft.json"))?;
    let plugin = json_boundary::read_json(&root.join(".codex-plugin/plugin.json"))?;
    let manifest_version = string(&manifest, "version");
    let plugin_version = string(&plugin, "version");
    if manifest_version.is_empty() || plugin_version.is_empty() {
        return Err("missing manifest or plugin version".to_string());
    }
    if manifest_version != plugin_version {
        return Err(format!(
            "manifest version {manifest_version} != plugin version {plugin_version}"
        ));
    }
    Ok(manifest_version.to_string())
}
