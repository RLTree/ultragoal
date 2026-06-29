use serde_json::{Value, json};

const RUNTIME_BINDING_DIGEST: &str =
    "sha256:0000000000000000000000000000000000000000000000000000000000000000";
const COVERAGE_MANIFESTS: &[&str] = &[
    ".harness/coverage-manifest.json",
    "templates/.harness/coverage-manifest.json",
];

pub(crate) fn stable_package_payload(rel: &str, bytes: &[u8]) -> Result<Vec<u8>, String> {
    if rel.starts_with("fixtures/valid/") && rel.ends_with(".json") {
        return stable_valid_fixture_payload(rel, bytes);
    }
    if COVERAGE_MANIFESTS.contains(&rel) {
        return stable_coverage_manifest_payload(rel, bytes);
    }
    Ok(bytes.to_vec())
}

fn stable_valid_fixture_payload(rel: &str, bytes: &[u8]) -> Result<Vec<u8>, String> {
    let mut value: Value = serde_json::from_slice(bytes)
        .map_err(|err| format!("{rel}: valid fixture digest canonicalization failed: {err}"))?;
    value["validator_receipt"] = json!({
        "schema": "harness-ultragoal.validator-receipt.v1",
        "runtime_binding": "audit_time_overlay"
    });
    Ok(value.to_string().into_bytes())
}

fn stable_coverage_manifest_payload(rel: &str, bytes: &[u8]) -> Result<Vec<u8>, String> {
    let mut value: Value = serde_json::from_slice(bytes)
        .map_err(|err| format!("{rel}: coverage manifest canonicalization failed: {err}"))?;
    value["repo_root_digest"] = json!(RUNTIME_BINDING_DIGEST);
    value["generated_at"] = json!("runtime_binding");
    if let Some(policy) = value
        .get_mut("changed_file_coupling_policy")
        .and_then(Value::as_object_mut)
    {
        policy.insert(
            "changed_files_digest".to_string(),
            json!(RUNTIME_BINDING_DIGEST),
        );
    }
    Ok(value.to_string().into_bytes())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn valid_fixture_runtime_receipts_are_stable() {
        let fixture = |run_id: &str| {
            json!({
                "schema":"fixture",
                "validator_receipt":{"schema":"harness-ultragoal.validator-receipt.v1","run_id":run_id},
                "content":{"claim":"same"}
            })
        };
        let first = super::stable_package_payload(
            "fixtures/valid/fixture.json",
            &serde_json::to_vec(&fixture("run-a")).expect("fixture a"),
        )
        .expect("stable a");
        let second = super::stable_package_payload(
            "fixtures/valid/fixture.json",
            &serde_json::to_vec(&fixture("run-b")).expect("fixture b"),
        )
        .expect("stable b");
        assert_eq!(first, second);
    }

    #[test]
    fn coverage_manifest_runtime_digests_are_stable() {
        let manifest = |source: &str, changed: &str| {
            json!({
                "schema":"harness-ultragoal.coverage-manifest.v1",
                "generated_at":"2026-01-01T00:00:00Z",
                "repo_root_digest":source,
                "changed_file_coupling_policy":{
                    "required":true,
                    "changed_files":["validator/src/lib.rs"],
                    "changed_files_digest":changed
                }
            })
        };
        let first = super::stable_package_payload(
            ".harness/coverage-manifest.json",
            &serde_json::to_vec(&manifest("sha256:a", "sha256:b")).expect("manifest a"),
        )
        .expect("stable a");
        let second = super::stable_package_payload(
            ".harness/coverage-manifest.json",
            &serde_json::to_vec(&manifest("sha256:c", "sha256:d")).expect("manifest b"),
        )
        .expect("stable b");
        assert_eq!(first, second);
    }

    #[test]
    fn coverage_manifest_payload_covers_missing_policy_and_malformed_json() {
        let without_policy = json!({
            "schema":"harness-ultragoal.coverage-manifest.v1",
            "generated_at":"2026-01-01T00:00:00Z",
            "repo_root_digest":"sha256:a"
        });
        assert!(
            super::stable_package_payload(
                "templates/.harness/coverage-manifest.json",
                &serde_json::to_vec(&without_policy).expect("manifest"),
            )
            .expect("stable without policy")
            .windows(super::RUNTIME_BINDING_DIGEST.len())
            .any(|window| window == super::RUNTIME_BINDING_DIGEST.as_bytes())
        );
        let err =
            super::stable_package_payload(".harness/coverage-manifest.json", b"{").unwrap_err();
        assert!(err.contains("coverage manifest canonicalization failed"));
    }
}
