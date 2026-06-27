use serde_json::{Value, json};
use std::path::Path;

pub(super) fn value(root: &Path, label: &str, rel: &str, expected: &str) -> Value {
    let path = root.join(rel);
    let mut failures = Vec::new();
    if crate::package::inventory::package_path_error(root, rel).is_some() {
        failures.push("path_not_package_safe".to_string());
    }
    let digest = match crate::digest::file(&path) {
        Ok(digest) => Some(digest),
        Err(err) => {
            failures.push(format!("digest_unavailable:{err}"));
            None
        }
    };
    let parsed = match crate::json_boundary::read_json(&path) {
        Ok(value) => Some(value),
        Err(err) => {
            failures.push(format!("json_unavailable:{err}"));
            None
        }
    };
    let status = parsed.as_ref().and_then(typed_status);
    if status.as_deref() != Some("pass") {
        failures.push("status_not_pass".to_string());
    }
    let candidate = parsed.as_ref().and_then(candidate_digest);
    if candidate.as_deref() != Some(expected) {
        failures.push("candidate_digest_mismatch".to_string());
    }
    json!({
        "label": label,
        "path": rel,
        "exists": path.is_file(),
        "digest": digest,
        "schema": parsed.as_ref().and_then(|value| value.get("schema")).and_then(Value::as_str),
        "status": status,
        "candidate_digest": candidate,
        "same_candidate": candidate.as_deref() == Some(expected),
        "failures": failures
    })
}

fn typed_status(value: &Value) -> Option<String> {
    if let Some(status) = value.get("status").and_then(Value::as_str) {
        return Some(status.to_string());
    }
    if value.get("schema").and_then(Value::as_str) == Some("harness-ultragoal.coverage-receipt.v1")
        && value.get("command_exit").and_then(Value::as_i64) == Some(0)
        && value.pointer("/coverage/percent").and_then(Value::as_f64) == Some(100.0)
        && value
            .get("uncovered_records")
            .and_then(Value::as_array)
            .is_some_and(Vec::is_empty)
        && value.get("claim_ceiling").and_then(Value::as_str) == Some("supports_complete_claim")
    {
        return Some("pass".to_string());
    }
    None
}

fn candidate_digest(value: &Value) -> Option<String> {
    [
        "/candidate_digest",
        "/target_revision/value",
        "/digests/candidate",
        "/target_digest",
    ]
    .into_iter()
    .find_map(|ptr| {
        value
            .pointer(ptr)
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn evidence_item_records_unsafe_package_paths() {
        let root = crate::self_tests::boundaries::support::temp_root("cli-evidence-item-path");
        std::fs::create_dir_all(&root).expect("root");
        let value = super::value(
            &root,
            "unsafe_receipt",
            "../outside-receipt.json",
            &crate::self_tests::boundaries::support::sha('a'),
        );
        let failures = value["failures"].as_array().expect("failures");
        assert!(
            failures
                .iter()
                .any(|failure| failure.as_str() == Some("path_not_package_safe"))
        );
        if root.exists() {
            std::fs::remove_dir_all(root).expect("cleanup cli evidence item");
        }
    }
}
