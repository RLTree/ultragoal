use crate::audit::AuditOptions;
use crate::json_boundary;
use crate::schema_catalog;
use crate::target_fixtures;
use crate::target_repo;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;

pub fn collect(
    options: &AuditOptions,
    red_report: &Path,
    validator_artifacts: &[Value],
    failures: &mut BTreeMap<String, Vec<String>>,
) -> Vec<Value> {
    match target_fixtures::write_target_receipts(
        &options.root,
        red_report.parent().unwrap_or(&options.root),
        validator_artifacts,
    ) {
        Ok(rows) => rows,
        Err(err) => {
            push_failure(failures, "target-repo-audit-capability", err);
            Vec::new()
        }
    }
}

pub fn validate(
    store: &schema_catalog::SchemaStore,
    failures: &mut BTreeMap<String, Vec<String>>,
    target_artifacts: &[Value],
) {
    for artifact in target_artifacts {
        validate_one(store, failures, artifact);
    }
}

fn validate_one(
    store: &schema_catalog::SchemaStore,
    failures: &mut BTreeMap<String, Vec<String>>,
    artifact: &Value,
) {
    let Some(path) = artifact.get("path").and_then(Value::as_str) else {
        return;
    };
    let Ok(value) = json_boundary::read_json(Path::new(path)) else {
        push_failure(
            failures,
            "target-repo-audit-capability",
            format!("{path}: unreadable target receipt"),
        );
        return;
    };
    let mut errors =
        schema_catalog::schema_errors(store, "target-repo-receipt.schema.json", &value);
    errors.extend(target_repo::target_receipt_errors(&value));
    if let Some(first) = errors.first() {
        push_failure(
            failures,
            "target-repo-audit-capability",
            format!("{path}: {first}"),
        );
    }
}

fn push_failure(
    failures: &mut BTreeMap<String, Vec<String>>,
    check_id: &str,
    detail: impl Into<String>,
) {
    failures
        .entry(check_id.to_string())
        .or_default()
        .push(detail.into());
}
