use crate::claim_semantics;
use crate::json_boundary;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::Path;

pub(crate) fn checks(
    root: &Path,
    validator_digests: &BTreeMap<String, String>,
    validator_artifacts: &[Value],
    failures: &mut BTreeMap<String, Vec<String>>,
) {
    let Ok(entries) = std::fs::read_dir(root.join("fixtures/valid")) else {
        push_failure(failures, "schema-valid", "fixtures/valid unreadable");
        return;
    };
    for entry in entries.flatten() {
        check(
            root,
            validator_digests,
            validator_artifacts,
            failures,
            &entry.path(),
        );
    }
}

pub(crate) fn check(
    root: &Path,
    validator_digests: &BTreeMap<String, String>,
    validator_artifacts: &[Value],
    failures: &mut BTreeMap<String, Vec<String>>,
    path: &Path,
) {
    if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
        return;
    }
    let rel = path
        .strip_prefix(root)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| path.display().to_string());
    match json_boundary::read_json(path) {
        Ok(value) => {
            let runtime_bound = runtime_bound_fixture(root, &value, validator_artifacts);
            push_semantic_failures(root, validator_digests, failures, &rel, &runtime_bound);
        }
        Err(err) => push_failure(
            failures,
            "schema-valid",
            format!("{rel}: json_load_failed: {err}"),
        ),
    }
}

fn runtime_bound_fixture(root: &Path, value: &Value, validator_artifacts: &[Value]) -> Value {
    let mut bound = value.clone();
    let run_id = bound
        .pointer("/ready_for_merge/validator_run_id")
        .and_then(Value::as_str)
        .or_else(|| {
            bound
                .pointer("/validator_receipt/run_id")
                .and_then(Value::as_str)
        })
        .unwrap_or("fixture-runtime");
    bound["validator_receipt"] = json!({
        "schema": "harness-ultragoal.validator-receipt.v1",
        "status": "pass",
        "run_id": run_id,
        "target_revision": {
            "kind": "package_digest",
            "value": crate::package::inventory::package_digest(root).unwrap_or_default()
        },
        "claim_ceiling": "source_audit_pass_source_local_only",
        "supported_claim_classes": ["source_local_audit_checks", "red_fixture_report"],
        "blocked_claim_classes": [
            "completion",
            "package_readiness",
            "review_readiness",
            "release_readiness",
            "final_packet_correctness",
            "update_goal_eligibility",
            "app_registry_or_reviewer_exposure"
        ],
        "validator_execution": {
            "command": {
                "command": "target/debug/ultragoal --root . source audit --receipt validation_artifacts/ultragoal-audit/validator-receipt.json --red-report validation_artifacts/ultragoal-audit/red-fixture-report.json"
            },
            "validator_artifacts": validator_artifacts
        },
        "generated_artifacts": ready_artifacts(root, run_id)
    });
    bound
}

pub(crate) fn ready_artifacts(root: &Path, run_id: &str) -> Vec<Value> {
    let Ok(entries) = std::fs::read_dir(root.join("examples/generated")) else {
        return Vec::new();
    };
    let mut out = entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let name = path.file_name()?.to_str()?;
            if !name.starts_with("READY_FOR_MERGE") || !name.ends_with(".json") {
                return None;
            }
            let rel = path
                .strip_prefix(root)
                .ok()?
                .to_string_lossy()
                .replace('\\', "/");
            Some(json!({
                "artifact_type": "ready_for_merge",
                "path": rel,
                "digest": crate::digest::file(&path).unwrap_or_default(),
                "validator_run_id": run_id
            }))
        })
        .collect::<Vec<_>>();
    out.sort_by(|a, b| {
        a.get("path")
            .and_then(Value::as_str)
            .cmp(&b.get("path").and_then(Value::as_str))
    });
    out
}

fn push_semantic_failures(
    root: &Path,
    validator_digests: &BTreeMap<String, String>,
    failures: &mut BTreeMap<String, Vec<String>>,
    rel: &str,
    value: &Value,
) {
    for failure in claim_semantics::semantic_failures(value, root, validator_digests) {
        push_failure(
            failures,
            &failure.check_id,
            format!("{rel}: {}: {}", failure.error, failure.detail),
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
