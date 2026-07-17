use crate::digest;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub(crate) fn ref_value_cached(
    root: &Path,
    rel: &str,
    digest_cache: &mut BTreeMap<String, String>,
) -> Value {
    json!({"path": rel, "digest": digest_or_zero_cached(root, rel, digest_cache)})
}

pub(crate) fn digest_or_zero(root: &Path, rel: &str) -> String {
    crate::package::inventory::resolve(root, rel)
        .ok()
        .and_then(|path| crate::audit::artifacts::artifact_digest_or_zero(&path).ok())
        .unwrap_or_else(|| digest::ZERO.to_string())
}

pub(crate) fn digest_or_zero_cached(
    root: &Path,
    rel: &str,
    digest_cache: &mut BTreeMap<String, String>,
) -> String {
    if let Some(digest) = digest_cache.get(rel) {
        return digest.clone();
    }
    let digest = digest_or_zero(root, rel);
    digest_cache.insert(rel.to_string(), digest.clone());
    digest
}

#[cfg(test)]
pub(crate) fn generated_artifacts(root: &Path, run_id: &str) -> Vec<Value> {
    let mut digest_cache = BTreeMap::new();
    generated_artifacts_cached(root, run_id, &mut digest_cache)
}

pub(crate) fn generated_artifacts_cached(
    root: &Path,
    run_id: &str,
    digest_cache: &mut BTreeMap<String, String>,
) -> Vec<Value> {
    let mut rows = BTreeMap::new();
    for suffix in required_generated_suffixes(root) {
        let path = generated_path_for_suffix(&suffix);
        rows.insert(
            path.clone(),
            generated_artifact_cached(root, run_id, &path, digest_cache),
        );
    }
    for item in ready_artifacts_cached(root, run_id, digest_cache) {
        if let Some(path) = item.get("path").and_then(Value::as_str) {
            rows.entry(path.to_string()).or_insert(item);
        }
    }
    rows.into_values().take(48).collect()
}

fn ready_artifacts_cached(
    root: &Path,
    run_id: &str,
    digest_cache: &mut BTreeMap<String, String>,
) -> Vec<Value> {
    let generated_at = crate::audit::clock::now_iso();
    crate::audit::package::run::ready_artifacts(root, run_id)
        .into_iter()
        .map(|mut item| {
            item["input_digest"] = json!(digest_or_zero_cached(
                root,
                "fixtures/valid/minimal-goal-run.json",
                digest_cache
            ));
            item["generated_at"] = json!(generated_at);
            item
        })
        .collect()
}

fn required_generated_suffixes(root: &Path) -> BTreeSet<String> {
    let Ok(schema) =
        crate::json_boundary::read_json(&root.join("schemas/validator-receipt.schema.json"))
    else {
        return BTreeSet::new();
    };
    let mut out = BTreeSet::new();
    collect_generated_suffixes(&schema, &mut out);
    out
}

fn collect_generated_suffixes(value: &Value, out: &mut BTreeSet<String>) {
    if let Some(pattern) = value
        .pointer("/properties/path/pattern")
        .and_then(Value::as_str)
        && let Some(suffix) = generated_suffix(pattern)
    {
        out.insert(suffix);
    }
    match value {
        Value::Array(items) => {
            for item in items {
                collect_generated_suffixes(item, out);
            }
        }
        Value::Object(map) => {
            for item in map.values() {
                collect_generated_suffixes(item, out);
            }
        }
        _ => {}
    }
}

fn generated_suffix(pattern: &str) -> Option<String> {
    pattern
        .strip_prefix("(^|/)")
        .and_then(|value| value.strip_suffix('$'))
        .map(|value| value.replace("\\-", "-").replace("\\.", "."))
}

fn generated_path_for_suffix(suffix: &str) -> String {
    if suffix == "red-fixture-report.json" || suffix.starts_with("target-") {
        format!("validation_artifacts/ultragoal-audit/{suffix}")
    } else {
        suffix.to_string()
    }
}

fn generated_artifact_cached(
    root: &Path,
    run_id: &str,
    path: &str,
    digest_cache: &mut BTreeMap<String, String>,
) -> Value {
    json!({
        "artifact_type": artifact_type(path),
        "path": path,
        "digest": digest_or_zero_cached(root, path, digest_cache),
        "validator_run_id": run_id,
        "input_digest": digest_or_zero_cached(root, "templates/RED_FIXTURES.json", digest_cache),
        "generated_at": crate::audit::clock::now_iso()
    })
}

fn artifact_type(path: &str) -> &'static str {
    if path.ends_with("red-fixture-report.json") {
        "red_fixture_report"
    } else if path.contains("READY_FOR_MERGE") {
        "ready_for_merge"
    } else {
        "validator_receipt"
    }
}
