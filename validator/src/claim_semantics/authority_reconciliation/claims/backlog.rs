use crate::audit::contract::Failure;
use crate::claim_semantics::str_field;
use serde_json::Value;
use std::path::Path;

pub(super) fn check(backlog: &Value, root: &Path, out: &mut Vec<Failure>) {
    for row in backlog
        .get("rows")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        check_ref(row.get("evidence"), root, out);
        for attempt in row
            .get("attempts")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            check_ref(attempt.get("evidence"), root, out);
        }
    }
}

pub(super) fn payload_refs(registry: &Value, root: &Path, out: &mut Vec<Failure>) {
    for row in registry
        .pointer("/root_freeze/payload_refs")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let path = str_field(row, "path");
        let Some(file) = super::super::safe_repo_path(root, &path) else {
            out.push(Failure::new(
                "authority-freeze",
                "payload_ref_path_unsafe",
                path,
            ));
            continue;
        };
        if str_field(row, "digest") != crate::digest::file(&file).unwrap_or_default() {
            out.push(Failure::new(
                "authority-freeze",
                "payload_ref_digest_mismatch",
                path,
            ));
        }
    }
}

fn check_ref(reference: Option<&Value>, root: &Path, out: &mut Vec<Failure>) {
    let path = reference
        .and_then(|row| row.get("path"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    let digest = reference
        .and_then(|row| row.get("digest"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    let Some(file) = super::super::safe_repo_path(root, path) else {
        out.push(Failure::new(
            "authority-freeze",
            "backlog_evidence_path_unsafe",
            path,
        ));
        return;
    };
    if digest != crate::digest::file(&file).unwrap_or_default() {
        out.push(Failure::new(
            "authority-freeze",
            "backlog_evidence_digest_mismatch",
            path,
        ));
    }
}
