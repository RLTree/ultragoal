use serde_json::{Value, json};
use std::path::Path;

const FIT: &str = "fit-repo-receipt.json";
const PRODUCT: &str = "product-fitness-receipt.json";
const JOURNEY: &str = "plugin-product-journey-receipt.json";
const SOURCE_DIR: &str = "validation_artifacts/harness";

pub(crate) fn mint_all(root: &Path, rel_dir: &Path, out_dir: &Path) -> Result<Value, String> {
    let candidate = crate::package::inventory::package_digest(root)?;
    let generated_at = crate::audit::clock::now_iso();
    let roundtrip_path = out_dir.join(FIT);
    let product_path = out_dir.join(PRODUCT);
    let journey_path = out_dir.join(JOURNEY);

    let fit = fit_receipt(root, &candidate, &generated_at)?;
    crate::json_boundary::write_json(&roundtrip_path, &fit)?;

    let product = product_receipt(root, &candidate, &generated_at)?;
    crate::json_boundary::write_json(&product_path, &product)?;

    let journey = journey_receipt(root, rel_dir, &candidate, &generated_at)?;
    crate::json_boundary::write_json(&journey_path, &journey)?;

    Ok(report(root, rel_dir, &fit, &product, &journey))
}

fn fit_receipt(root: &Path, candidate: &str, generated_at: &str) -> Result<Value, String> {
    let mut value = read_template(root, FIT)?;
    set_common(&mut value, candidate, generated_at);
    set_plugin_version(root, &mut value)?;
    refresh_artifact_refs(root, &mut value)?;
    set_digest(&mut value, crate::audit::fit_repo_receipt::canonical_digest);
    Ok(value)
}

fn product_receipt(root: &Path, candidate: &str, generated_at: &str) -> Result<Value, String> {
    let mut value = read_template(root, PRODUCT)?;
    set_common(&mut value, candidate, generated_at);
    refresh_artifact_refs(root, &mut value)?;
    set_digest(
        &mut value,
        crate::audit::product::fitness::receipt::canonical_digest,
    );
    Ok(value)
}

fn journey_receipt(
    root: &Path,
    rel_dir: &Path,
    candidate: &str,
    generated_at: &str,
) -> Result<Value, String> {
    let mut value = read_template(root, JOURNEY)?;
    set_common(&mut value, candidate, generated_at);
    refresh_artifact_refs(root, &mut value)?;
    let rel_fit = rel_dir.join(FIT).to_string_lossy().to_string();
    let first = value
        .get_mut("evidence")
        .and_then(Value::as_array_mut)
        .and_then(|items| items.first_mut())
        .ok_or_else(|| "plugin_product_journey_receipt_missing_fit_evidence".to_string())?;
    first["path"] = json!(rel_fit);
    first["digest"] = json!(crate::digest::file(&root.join(rel_dir).join(FIT))?);
    Ok(value)
}

fn read_template(root: &Path, file: &str) -> Result<Value, String> {
    crate::json_boundary::read_json(&root.join(SOURCE_DIR).join(file))
}

fn set_common(value: &mut Value, candidate: &str, generated_at: &str) {
    value["generated_at"] = json!(generated_at);
    value["target_revision"] = json!({"kind": "package_digest", "value": candidate});
}

fn set_plugin_version(root: &Path, value: &mut Value) -> Result<(), String> {
    let plugin = crate::json_boundary::read_json(&root.join(".codex-plugin/plugin.json"))?;
    let version = plugin
        .get("version")
        .and_then(Value::as_str)
        .ok_or_else(|| ".codex-plugin/plugin.json: missing version".to_string())?;
    value["plugin_version"] = json!(version);
    value["cache_package_path"] =
        json!(format!("local-harness-plugins/harness-ultragoal/{version}"));
    Ok(())
}

fn set_digest(value: &mut Value, digest: impl Fn(&Value) -> String) {
    value["receipt_digest"] = json!(crate::digest::ZERO);
    value["receipt_digest"] = json!(digest(value));
}

fn refresh_artifact_refs(root: &Path, value: &mut Value) -> Result<(), String> {
    match value {
        Value::Object(map) => {
            if let Some(path) = map
                .get("path")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
                && map.contains_key("digest")
            {
                if let Some(error) = crate::package::inventory::package_path_error(root, &path) {
                    return Err(format!("{path}: invalid artifact path: {error}"));
                }
                map.insert(
                    "digest".to_string(),
                    Value::String(crate::digest::file(&root.join(path))?),
                );
            }
            for child in map.values_mut() {
                refresh_artifact_refs(root, child)?;
            }
        }
        Value::Array(items) => {
            for child in items {
                refresh_artifact_refs(root, child)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn report(root: &Path, rel_dir: &Path, fit: &Value, product: &Value, journey: &Value) -> Value {
    let mut failures = Vec::new();
    failures.extend(
        crate::audit::fit_repo_receipt::failures(root, fit)
            .into_iter()
            .map(|item| format!("fit_repo:{item}")),
    );
    failures.extend(
        crate::audit::product::fitness::canonical_package_receipt_value_failures(root, product)
            .into_iter()
            .map(|item| format!("product_fitness:{item}")),
    );
    failures.extend(
        crate::audit::plugin::product::cohesion::journey_value_failures(root, journey)
            .into_iter()
            .map(|item| format!("product_journey:{item}")),
    );
    let status = if failures.is_empty() { "pass" } else { "fail" };
    json!({
        "schema": "harness-ultragoal.product-receipt-mint-report.v1",
        "status": status,
        "target_revision": {
            "kind": "package_digest",
            "value": crate::package::inventory::package_digest(root).unwrap_or_default()
        },
        "receipts": [
            row(root, rel_dir, FIT, fit, status),
            row(root, rel_dir, PRODUCT, product, status),
            row(root, rel_dir, JOURNEY, journey, status)
        ],
        "failures": failures,
        "claim_ceiling": "source_local_product_receipts_only"
    })
}

fn row(root: &Path, rel_dir: &Path, file: &str, value: &Value, status: &str) -> Value {
    let rel = rel_dir.join(file).to_string_lossy().to_string();
    crate::cli::product::receipt_row(
        &rel,
        &crate::digest::file(&root.join(&rel)).unwrap_or_else(|_| crate::digest::ZERO.to_string()),
        value
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or(status),
    )
}

#[cfg(test)]
mod tests;
