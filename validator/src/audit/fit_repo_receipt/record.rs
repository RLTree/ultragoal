use crate::package::artifact::refs::ArtifactRef;
use serde_json::Value;

pub(super) const FIT_REPO_ENTRYPOINT: &str = "harness-ultragoal:fit-repo";

pub(super) struct FitRepoReceipt {
    pub(super) schema: String,
    pub(super) target_revision_value: String,
    pub(super) plugin_source_path: String,
    pub(super) installed_plugin_path: String,
    pub(super) cache_package_path: String,
    pub(super) plugin_version: String,
    pub(super) entrypoint_id: String,
    pub(super) target_classification: String,
    pub(super) runtime_surface_classification: String,
    pub(super) product_surface_classification: String,
    pub(super) check_artifacts: Vec<Result<ArtifactRef, String>>,
    pub(super) check_count: usize,
    pub(super) blocker_owners: Vec<String>,
    pub(super) claim_ceiling: String,
    pub(super) producer_actor_id: String,
    pub(super) receipt_digest: String,
}

impl FitRepoReceipt {
    pub(super) fn from_value(value: &Value) -> Self {
        Self {
            schema: field(value, "schema"),
            target_revision_value: pointer(value, "/target_revision/value"),
            plugin_source_path: field(value, "plugin_source_path"),
            installed_plugin_path: field(value, "installed_plugin_path"),
            cache_package_path: field(value, "cache_package_path"),
            plugin_version: field(value, "plugin_version"),
            entrypoint_id: pointer(value, "/entrypoint_contract/id"),
            target_classification: field(value, "target_classification"),
            runtime_surface_classification: field(value, "runtime_surface_classification"),
            product_surface_classification: field(value, "product_surface_classification"),
            check_artifacts: check_artifacts(value),
            check_count: value
                .get("checks")
                .and_then(Value::as_array)
                .map_or(0, Vec::len),
            blocker_owners: value
                .get("blockers")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .map(|blocker| field(blocker, "owner"))
                .collect(),
            claim_ceiling: field(value, "claim_ceiling"),
            producer_actor_id: field(value, "producer_actor_id"),
            receipt_digest: field(value, "receipt_digest"),
        }
    }
}

fn check_artifacts(value: &Value) -> Vec<Result<ArtifactRef, String>> {
    value
        .get("checks")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .flat_map(|check| {
            ["stdout", "stderr"]
                .into_iter()
                .filter_map(|key| check.get(key))
        })
        .map(|artifact| ArtifactRef::from_object(artifact, "fit-repo command artifact"))
        .collect()
}

fn field(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn pointer(value: &Value, pointer: &str) -> String {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}
