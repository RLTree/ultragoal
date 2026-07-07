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
    let label_failures = parsed
        .as_ref()
        .map(|value| super::receipt_evaluation::label_failures(root, label, value, expected))
        .unwrap_or_default();
    let status = parsed
        .as_ref()
        .and_then(|value| super::receipt_evaluation::typed_status(label, value, &label_failures));
    if status != Some("pass") {
        failures.push("status_not_pass".to_string());
    }
    let candidate = parsed.as_ref().and_then(candidate_digest);
    if candidate.as_deref() != Some(expected) {
        failures.push("candidate_digest_mismatch".to_string());
    }
    failures.extend(label_failures);
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
        let root =
            crate::self_tests::boundaries::workspace_fixtures::temp_root("cli-evidence-item-path");
        std::fs::create_dir_all(&root).expect("root");
        let value = super::value(
            &root,
            "unsafe_receipt",
            "../outside-receipt.json",
            &crate::self_tests::boundaries::workspace_fixtures::sha('a'),
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

    #[test]
    fn evidence_item_infers_exact_coverage_receipt_status() {
        let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
            "cli-evidence-item-coverage",
        );
        std::fs::create_dir_all(root.join("validation_artifacts/coverage")).expect("coverage dir");
        let candidate = crate::self_tests::boundaries::workspace_fixtures::sha('c');
        let rel = "validation_artifacts/coverage/coverage-receipt.json";
        crate::json_boundary::write_json(
            &root.join(rel),
            &serde_json::json!({
                "schema": "harness-ultragoal.coverage-receipt.v1",
                "coverage_target_dir": "target/ultragoal-coverage",
                "command_exit": 0,
                "coverage": {"percent": 100.0},
                "uncovered_records": [],
                "claim_ceiling": "supports_complete_coverage_claim",
                "supported_claim_classes": ["complete_coverage"],
                "blocked_claim_classes": [
                    "completion",
                    "package_readiness",
                    "review_readiness",
                    "release_readiness",
                    "final_packet_correctness",
                    "update_goal_eligibility",
                    "app_registry_or_reviewer_exposure"
                ],
                "target_revision": {"kind": "package_digest", "value": candidate}
            }),
        )
        .expect("coverage receipt");
        let value = super::value(&root, "coverage", rel, &candidate);
        assert_eq!(value["status"], "pass");
        assert!(value["failures"].as_array().expect("failures").is_empty());
        std::fs::remove_dir_all(root).expect("cleanup cli evidence coverage");
    }

    #[test]
    fn evidence_item_refuses_status_inference_for_nonpassing_typed_receipts() {
        let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
            "cli-evidence-item-no-status",
        );
        std::fs::create_dir_all(root.join("validation_artifacts/coverage")).expect("coverage dir");
        let candidate = crate::self_tests::boundaries::workspace_fixtures::sha('d');
        let rel = "validation_artifacts/coverage/coverage-receipt.json";
        crate::json_boundary::write_json(
            &root.join(rel),
            &serde_json::json!({
                "schema": "harness-ultragoal.coverage-receipt.v1",
                "coverage_target_dir": "target/ultragoal-coverage",
                "command_exit": 0,
                "coverage": {"percent": 99.0},
                "uncovered_records": [{"path": "src/lib.rs"}],
                "claim_ceiling": "withheld_or_blocked",
                "target_revision": {"kind": "package_digest", "value": candidate}
            }),
        )
        .expect("coverage receipt");
        let value = super::value(&root, "coverage", rel, &candidate);
        assert_eq!(value["status"], serde_json::Value::Null);
        assert!(
            value["failures"]
                .as_array()
                .expect("failures")
                .iter()
                .any(|failure| failure.as_str() == Some("status_not_pass"))
        );
        std::fs::remove_dir_all(root).expect("cleanup cli evidence no status");
    }
}
