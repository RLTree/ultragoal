use serde_json::Value;
use std::path::Path;

pub(crate) fn validate_anchor_sources(
    validator_digest: &str,
    package_digest: &str,
    current_root: Option<&Path>,
    review: &Value,
    archive: &Value,
    out: &mut Vec<String>,
) {
    if let Some(root) = current_root {
        match crate::package::inventory::package_digest(root) {
            Ok(current) if current == package_digest => {}
            Ok(_) => out.push("validator package digest is stale for current root".to_string()),
            Err(err) => out.push(format!("current package digest failed: {err}")),
        }
    }
    if review.get("status").and_then(Value::as_str) != Some("pass") {
        out.push("review target receipt status is not pass".to_string());
    }
    if archive.get("status").and_then(Value::as_str) != Some("pass") {
        out.push("archive receipt status is not pass".to_string());
    }
    if string(review, "/package_digest") != package_digest {
        out.push("review target package digest does not match validator".to_string());
    }
    if string(review, "/validator_receipt/digest") != validator_digest {
        out.push("review target validator receipt digest does not match".to_string());
    }
    if string(archive, "/source/package_digest") != package_digest {
        out.push("archive source package digest does not match validator".to_string());
    }
    for (label, value) in [
        (
            "review target digest",
            string(review, "/review_target_digest"),
        ),
        ("archive digest", string(archive, "/archive/digest")),
    ] {
        if !is_sha256(&value) {
            out.push(format!("{label} is not a sha256 digest"));
        }
    }
}

fn is_sha256(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value
            .as_bytes()
            .iter()
            .skip(7)
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn string(value: &Value, pointer: &str) -> String {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}
