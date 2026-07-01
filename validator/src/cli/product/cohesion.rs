use serde_json::{Value, json};
use std::path::Path;

pub(super) fn prove(root: &Path) -> Result<Value, String> {
    let candidate = crate::package::inventory::package_digest(root)?;
    let mut checks = serde_json::Map::new();
    crate::target_repo::product::cohesion::check(
        root,
        &["product-cohesion".to_string()],
        true,
        &mut checks,
    );
    let row = checks
        .remove("product-cohesion")
        .ok_or_else(|| "product_cohesion_check_missing".to_string())?;
    let status = row.get("status").and_then(Value::as_str).unwrap_or("fail");
    let detail = row
        .get("detail")
        .and_then(Value::as_str)
        .unwrap_or("product cohesion check failed without detail");
    let failures = if status == "pass" {
        Vec::new()
    } else {
        vec![detail.to_string()]
    };
    Ok(json!({
        "schema": "harness-ultragoal.product-cohesion-prove-report.v1",
        "status": if status == "pass" { "pass" } else { "fail" },
        "target_revision": {
            "kind": "package_digest",
            "value": candidate
        },
        "required_paths": [
            "docs/product-cohesion.md",
            "validation_artifacts/product-cohesion/journey-receipt.json"
        ],
        "checks": {
            "product-cohesion": row
        },
        "failures": failures,
        "claim_ceiling": "source_local_product_cohesion_only"
    }))
}
