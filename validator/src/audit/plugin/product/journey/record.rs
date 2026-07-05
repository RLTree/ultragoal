use crate::package::artifact::refs::ArtifactRef;
use serde_json::Value;

pub(super) const FIT_REPO_RECEIPT: &str = "validation_artifacts/harness/fit-repo-receipt.json";

pub(super) struct PluginProductJourneyReceipt {
    pub(super) status: String,
    pub(super) target_revision_kind: String,
    pub(super) target_revision_value: String,
    pub(super) journey_step_count: usize,
    pub(super) evidence: Vec<JourneyEvidenceRef>,
    pub(super) error_path_evidence: Option<JourneyEvidenceRef>,
    pub(super) claim_ceiling: String,
    pub(super) generated_at: String,
}

#[derive(Clone, Debug)]
pub(super) struct JourneyEvidenceRef {
    pub(super) path: String,
    pub(super) artifact: Result<ArtifactRef, String>,
}

impl PluginProductJourneyReceipt {
    pub(super) fn from_value(value: &Value) -> Self {
        Self {
            status: field(value, "status"),
            target_revision_kind: pointer(value, "/target_revision/kind"),
            target_revision_value: pointer(value, "/target_revision/value"),
            journey_step_count: value
                .get("journey")
                .and_then(Value::as_array)
                .map_or(0, Vec::len),
            evidence: value
                .get("evidence")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .map(JourneyEvidenceRef::from_value)
                .collect(),
            error_path_evidence: value
                .get("error_path_evidence")
                .map(JourneyEvidenceRef::from_value),
            claim_ceiling: field(value, "claim_ceiling"),
            generated_at: field(value, "generated_at"),
        }
    }
}

impl JourneyEvidenceRef {
    fn from_value(value: &Value) -> Self {
        Self {
            path: field(value, "path"),
            artifact: ArtifactRef::from_object(value, "plugin journey evidence"),
        }
    }
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
