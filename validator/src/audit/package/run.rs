use crate::audit::package::{checks, outputs, targets};
use crate::audit::{AuditOptions, artifacts};
use crate::claim_semantics;
use crate::json_boundary;
use crate::schema_catalog;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub fn run(options: AuditOptions, red_report: PathBuf) -> Result<i32, String> {
    let start = crate::audit::clock::now_iso();
    let store = schema_catalog::load(&options.root);
    let check_ids = artifacts::check_ids(&store);
    let validator_artifacts = artifacts::validator_artifacts(&options.root)?;
    let validator_digests = artifacts::digest_map(&validator_artifacts);
    let mut failures = checks::checks(&options.root, &store, &check_ids, &validator_artifacts);
    collect_failures(
        &options,
        &store,
        &validator_digests,
        &validator_artifacts,
        &mut failures,
    );
    let red = collect_red(&options, &store, &validator_digests, &mut failures);
    let target_artifacts =
        targets::collect(&options, &red_report, &validator_artifacts, &mut failures);
    targets::validate(&store, &mut failures, &target_artifacts);
    checks::final_hygiene_check(&options.root, &mut failures);
    let status = outputs::package_status(&failures);
    let red_status = outputs::red_report_status(&red);
    outputs::write_red_report(&options.root, &red_report, red_status, &red)?;
    let (stdout, stderr) = outputs::write_stdio_receipts(&options.receipt, status, red.len())?;
    outputs::write_validator_receipt(outputs::ReceiptParts {
        options,
        red_report,
        stdout,
        stderr,
        check_ids,
        failures,
        red,
        target_artifacts,
        start,
        status,
        validator_artifacts,
    })
}

fn collect_failures(
    options: &AuditOptions,
    store: &schema_catalog::SchemaStore,
    validator_digests: &BTreeMap<String, String>,
    validator_artifacts: &[Value],
    failures: &mut BTreeMap<String, Vec<String>>,
) {
    for error in &store.errors {
        push_failure(
            failures,
            "schema-valid",
            format!("schema_bootstrap_failed: {error}"),
        );
    }
    semantic_valid_fixture_checks(
        &options.root,
        validator_digests,
        validator_artifacts,
        failures,
    );
}

fn collect_red(
    options: &AuditOptions,
    store: &schema_catalog::SchemaStore,
    validator_digests: &BTreeMap<String, String>,
    failures: &mut BTreeMap<String, Vec<String>>,
) -> BTreeMap<String, Value> {
    let red = crate::red::fixtures::red_fixture_results(&options.root, store, validator_digests);
    if red.is_empty() {
        push_failure(
            failures,
            "red-fixture-coverage",
            "red catalog or materialization unavailable",
        );
    }
    if red
        .values()
        .any(|row| row.get("status").and_then(Value::as_str) != Some("pass"))
    {
        push_failure(
            failures,
            "red-fixture-coverage",
            "one or more red fixtures did not fail as expected",
        );
    }
    red
}

pub(crate) fn semantic_valid_fixture_checks(
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
        semantic_valid_fixture_check(
            root,
            validator_digests,
            validator_artifacts,
            failures,
            &entry.path(),
        );
    }
}

pub(crate) fn semantic_valid_fixture_check(
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
