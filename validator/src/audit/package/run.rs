use crate::audit::package::{checks, outputs, targets};
use crate::audit::{AuditOptions, artifacts};
use crate::claim_semantics;
use crate::json_boundary;
use crate::schema_catalog;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub fn run(options: AuditOptions, red_report: PathBuf) -> Result<i32, String> {
    let start = crate::audit::clock::now_iso();
    let store = schema_catalog::load(&options.root);
    let check_ids = artifacts::check_ids(&store);
    let validator_artifacts = artifacts::validator_artifacts(&options.root)?;
    let validator_digests = artifacts::digest_map(&validator_artifacts);
    let mut failures = checks::checks(&options.root, &store, &check_ids, &validator_artifacts);
    collect_failures(&options, &store, &validator_digests, &mut failures);
    let red = collect_red(&options, &store, &validator_digests, &mut failures);
    let target_artifacts =
        targets::collect(&options, &red_report, &validator_artifacts, &mut failures);
    targets::validate(&store, &mut failures, &target_artifacts);
    checks::final_hygiene_check(&options.root, &mut failures);
    let status = outputs::package_status(&failures);
    outputs::write_red_report(&red_report, status, &red)?;
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
    failures: &mut BTreeMap<String, Vec<String>>,
) {
    for error in &store.errors {
        push_failure(
            failures,
            "schema-valid",
            format!("schema_bootstrap_failed: {error}"),
        );
    }
    semantic_valid_fixture_checks(&options.root, validator_digests, failures);
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
    failures: &mut BTreeMap<String, Vec<String>>,
) {
    let Ok(entries) = std::fs::read_dir(root.join("fixtures/valid")) else {
        push_failure(failures, "schema-valid", "fixtures/valid unreadable");
        return;
    };
    for entry in entries.flatten() {
        semantic_valid_fixture_check(root, validator_digests, failures, &entry.path());
    }
}

pub(crate) fn semantic_valid_fixture_check(
    root: &Path,
    validator_digests: &BTreeMap<String, String>,
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
        Ok(value) => push_semantic_failures(root, validator_digests, failures, &rel, &value),
        Err(err) => push_failure(
            failures,
            "schema-valid",
            format!("{rel}: json_load_failed: {err}"),
        ),
    }
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
