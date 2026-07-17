use crate::digest;
use serde_json::{Value, json};
use std::path::Path;

pub struct TargetReceiptInput<'a> {
    pub display_repo: &'a str,
    pub mode: &'a str,
    pub status: &'a str,
    pub checks: serde_json::Map<String, Value>,
    pub command_text: &'a str,
    pub command: Value,
    pub validator_artifacts: &'a [Value],
    pub markers: Vec<String>,
    pub require_observability: bool,
    pub require_product_cohesion: bool,
}

pub fn target_receipt(input: TargetReceiptInput<'_>) -> Value {
    let mut receipt = json!({
        "schema": "harness-ultragoal.target-repo-receipt.v1",
        "mode": input.mode,
        "target_repo": input.display_repo,
        "status": input.status,
        "requirements": {
            "observability_required": input.require_observability,
            "observability_reason": if input.require_observability { "requested" } else { "not_requested" },
            "product_cohesion_required": input.require_product_cohesion,
            "product_cohesion_reason": if input.require_product_cohesion { "requested" } else { "not_requested" }
        },
        "checks": Value::Object(input.checks),
        "command": input.command,
        "validator_artifacts": input.validator_artifacts,
        "gate_markers": input.markers,
        "invocation": input.command_text,
        "generated_at": crate::audit::clock::now_iso()
    });
    let fingerprint = canonical_fingerprint(&receipt);
    receipt["repo_fingerprint"] = json!(fingerprint);
    receipt
}

pub fn canonical_fingerprint(receipt: &Value) -> String {
    let mut payload = receipt.clone();
    if let Some(obj) = payload.as_object_mut() {
        obj.remove("repo_fingerprint");
        obj.remove("generated_at");
    }
    digest::canonical_json(&payload)
}

pub fn row(repo: &Path, status: &str, detail: &str, rel: Option<&str>) -> Value {
    json!({"status": status, "detail": detail, "evidence": artifact(repo, rel)})
}

pub fn artifact(repo: &Path, rel: Option<&str>) -> Value {
    let Some(rel) = rel else {
        return json!({"path": "<none>", "digest": digest::ZERO});
    };
    if let Ok(bytes) = crate::target_repo::safe_fs::read(repo, rel) {
        return json!({"path": rel, "digest": digest::bytes(&bytes)});
    }
    if let Ok(path) = crate::target_repo::safe_fs::dir_path(repo, rel) {
        return json!({"path": rel, "digest": super::baseline::directory_listing_digest(&path)});
    }
    json!({"path": rel, "digest": digest::ZERO})
}

pub fn overall_status(checks: &serde_json::Map<String, Value>) -> String {
    if checks
        .values()
        .any(|row| row.get("status").and_then(Value::as_str) == Some("fail"))
    {
        "fail".to_string()
    } else if checks
        .values()
        .any(|row| row.get("status").and_then(Value::as_str) == Some("blocked"))
    {
        "blocked".to_string()
    } else {
        "pass".to_string()
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn overall_status_projects_fail_blocked_and_pass() {
        let mut checks = serde_json::Map::new();
        checks.insert("a".to_string(), json!({"status":"fail"}));
        assert_eq!(super::overall_status(&checks), "fail");
        checks.insert("a".to_string(), json!({"status":"blocked"}));
        assert_eq!(super::overall_status(&checks), "blocked");
        checks.insert("a".to_string(), json!({"status":"pass"}));
        assert_eq!(super::overall_status(&checks), "pass");
    }
}
