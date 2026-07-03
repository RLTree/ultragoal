use crate::json_boundary;
use serde_json::Value;
use std::path::Path;

pub(crate) const STALE_ARTIFACT_RESOURCE: &str = "stale_artifact_resource_packaged";
pub(crate) const ROOT_VERIFICATION_RECEIPT_RESOURCE: &str =
    "root_verification_receipt_resource_packaged";
pub(crate) const FIXTURE_SUPPORT_ACTIVE_ARTIFACT: &str =
    "fixture_support_resource_packaged_as_active_artifact";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResourcePurposeFailure {
    pub code: &'static str,
    pub detail: String,
}

pub(crate) fn failures(root: &Path, manifest: &Value) -> Vec<ResourcePurposeFailure> {
    let mut out = Vec::new();
    for rel in crate::package::inventory::inventory_paths(manifest) {
        if let Some(failure) = path_failure(&rel) {
            out.push(failure);
            continue;
        }
        if rel.starts_with("artifacts/")
            && rel.ends_with(".json")
            && active_artifact_is_fixture_support(root, &rel)
        {
            out.push(ResourcePurposeFailure {
                code: FIXTURE_SUPPORT_ACTIVE_ARTIFACT,
                detail: rel,
            });
        }
    }
    out
}

fn path_failure(rel: &str) -> Option<ResourcePurposeFailure> {
    if rel.starts_with("artifacts/") && path_part_contains_stale(rel) {
        return Some(ResourcePurposeFailure {
            code: STALE_ARTIFACT_RESOURCE,
            detail: rel.to_string(),
        });
    }
    if rel.starts_with("artifacts/root-receipts/") && rel.ends_with(".json") {
        return Some(ResourcePurposeFailure {
            code: ROOT_VERIFICATION_RECEIPT_RESOURCE,
            detail: rel.to_string(),
        });
    }
    None
}

fn path_part_contains_stale(rel: &str) -> bool {
    rel.split('/')
        .any(|part| part.to_ascii_lowercase().contains("stale"))
}

fn active_artifact_is_fixture_support(root: &Path, rel: &str) -> bool {
    let Ok(path) = crate::package::inventory::resolve(root, rel) else {
        return false;
    };
    let Ok(value) = json_boundary::read_json(&path) else {
        return false;
    };
    value.get("fixture_purpose").is_some() || value.get("fixture_only").is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_active_artifact_path_is_rejected() {
        let failure = path_failure("artifacts/root-receipts/stale-proof.json");

        assert_eq!(failure.unwrap().code, STALE_ARTIFACT_RESOURCE);
    }

    #[test]
    fn fixture_support_path_is_not_active_artifact() {
        let failure = path_failure("fixtures/root-receipts/stale-proof.fixture.json");

        assert!(failure.is_none());
    }

    #[test]
    fn active_root_verification_receipt_path_is_rejected() {
        let failure = path_failure("artifacts/root-receipts/post-merge.json");

        assert_eq!(failure.unwrap().code, ROOT_VERIFICATION_RECEIPT_RESOURCE);
    }

    #[test]
    fn active_artifact_fixture_support_is_rejected_but_missing_or_malformed_is_not_substituted() {
        let root =
            crate::self_tests::boundaries::support::temp_root("resource-purpose-active-artifact");
        std::fs::create_dir_all(root.join("artifacts")).expect("artifacts dir");
        std::fs::write(
            root.join("artifacts/fixture.json"),
            r#"{"fixture_only":true}"#,
        )
        .expect("fixture artifact");
        std::fs::write(root.join("artifacts/malformed.json"), "{").expect("malformed artifact");
        let manifest = serde_json::json!({
            "resources":[
                "artifacts/fixture.json",
                "artifacts/../escape.json",
                "artifacts/missing.json",
                "artifacts/malformed.json"
            ]
        });
        let failures = failures(&root, &manifest);
        assert_eq!(failures.len(), 1, "{failures:?}");
        assert_eq!(failures[0].code, FIXTURE_SUPPORT_ACTIVE_ARTIFACT);
        assert_eq!(failures[0].detail, "artifacts/fixture.json");
        std::fs::remove_dir_all(root).expect("cleanup resource purpose");
    }
}
