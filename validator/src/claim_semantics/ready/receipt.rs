use crate::audit::contract::Failure;
use crate::claim_semantics::str_field;
use crate::digest;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub fn check_validator_receipt(
    bundle: &Value,
    ready: &Value,
    validator_digests: &BTreeMap<String, String>,
    out: &mut Vec<Failure>,
) {
    if ready
        .pointer("/commands/0/command")
        .and_then(Value::as_str)
        .unwrap_or("")
        .contains("not_run")
    {
        out.push(Failure::new(
            "command-evidence",
            "not_run_is_not_evidence",
            "ready command",
        ));
    }
    let got = validator_artifacts(bundle);
    let missing = validator_digests
        .keys()
        .filter(|path| !got.contains_key(*path))
        .cloned()
        .collect::<Vec<_>>();
    let stale = validator_digests
        .iter()
        .filter(|(path, want)| {
            got.get(*path)
                .is_some_and(|have| have != *want && have != digest::ZERO)
        })
        .map(|(path, _)| path.clone())
        .collect::<Vec<_>>();
    if !missing.is_empty() || !stale.is_empty() {
        out.push(Failure::new(
            "validator-execution-provenance",
            "validator_receipt_not_runtime_provenance",
            [missing, stale].concat().join(","),
        ));
    }
    pass_receipt_artifact_check(bundle, &got, validator_digests, out);
    validator_run_check(bundle, ready, out);
}

pub fn check_ready_receipt_set(bundle: &Value, ready_receipts: &[Value], out: &mut Vec<Failure>) {
    let mut seen = BTreeSet::new();
    for receipt in ready_receipts {
        let lane_id = str_field(receipt, "lane_id");
        if !seen.insert(lane_id.clone()) {
            out.push(Failure::new(
                "ready-receipt-provenance",
                "ready_receipt_not_lane_bound",
                format!("{lane_id}:duplicate ready receipt"),
            ));
        }
        validator_run_check(bundle, receipt, out);
    }
}

fn validator_artifacts(bundle: &Value) -> BTreeMap<String, String> {
    bundle
        .pointer("/validator_receipt/validator_execution/validator_artifacts")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|row| (str_field(row, "path"), str_field(row, "digest")))
        .collect()
}

pub(crate) fn generated_ready_artifact_ok(bundle: &Value, receipt: &Value, root: &Path) -> bool {
    let lane_id = str_field(receipt, "lane_id");
    let mut exact_matches = 0;
    for artifact in ready_artifacts(bundle, receipt) {
        if artifact_ref_invalid(root, artifact) {
            return false;
        }
        let path = artifact.get("path").and_then(Value::as_str).unwrap_or("");
        let Ok(value) = crate::json_boundary::read_json(&root.join(path)) else {
            return false;
        };
        if str_field(&value, "lane_id") == lane_id && value == *receipt {
            exact_matches += 1;
        }
    }
    exact_matches == 1
}

fn ready_artifacts<'a>(bundle: &'a Value, receipt: &Value) -> Vec<&'a Value> {
    let run_id = str_field(receipt, "validator_run_id");
    bundle
        .pointer("/validator_receipt/generated_artifacts")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|row| {
            str_field(row, "artifact_type") == "ready_for_merge"
                && str_field(row, "validator_run_id") == run_id
        })
        .collect()
}

fn artifact_ref_invalid(root: &Path, artifact: &Value) -> bool {
    let rel = artifact.get("path").and_then(Value::as_str).unwrap_or("");
    if crate::package::inventory::package_path_error(root, rel).is_some() {
        return true;
    }
    let digest_value = artifact.get("digest").and_then(Value::as_str).unwrap_or("");
    digest_value == digest::ZERO
        || !digest_value.starts_with("sha256:")
        || digest::file(&root.join(rel)).map_or(true, |actual| actual != digest_value)
}

pub(crate) fn ready_output_failure(lane_id: &str) -> Failure {
    Failure::new(
        "ready-receipt-provenance",
        "ready_receipt_not_validator_output",
        lane_id,
    )
}

fn pass_receipt_artifact_check(
    bundle: &Value,
    got: &BTreeMap<String, String>,
    validator_digests: &BTreeMap<String, String>,
    out: &mut Vec<Failure>,
) {
    if bundle
        .pointer("/validator_receipt/status")
        .and_then(Value::as_str)
        == Some("pass")
        && validator_digests
            .iter()
            .any(|(path, digest)| got.get(path) != Some(digest))
    {
        out.push(Failure::new(
            "validator-artifact-provenance",
            "validator_artifact_not_current",
            "validator bundle",
        ));
    }
}

pub(super) fn validator_run_check(bundle: &Value, ready: &Value, out: &mut Vec<Failure>) {
    let expected = bundle
        .pointer("/validator_receipt/run_id")
        .and_then(Value::as_str)
        .unwrap_or("");
    if str_field(&ready["provenance"], "validator_run_id") != str_field(ready, "validator_run_id")
        || (ready.get("ready").and_then(Value::as_bool) == Some(true)
            && str_field(ready, "validator_run_id") != expected)
    {
        out.push(Failure::new(
            "ready-receipt-provenance",
            "ready_receipt_not_validator_output",
            "validator run mismatch",
        ));
    }
    let command = bundle
        .pointer("/validator_receipt/validator_execution/command/command")
        .and_then(Value::as_str)
        .unwrap_or("");
    if bundle
        .pointer("/validator_receipt/status")
        .and_then(Value::as_str)
        == Some("pass")
        && !command.contains("--root")
    {
        out.push(Failure::new(
            "command-evidence",
            "validator_command_missing_full_argv",
            "validator command",
        ));
    }
}
