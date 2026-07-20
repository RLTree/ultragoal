use super::*;

/// Authenticated continuity classification. Durable custody keeps the record,
/// key, and grant material private; callers receive only the safe next action.
#[derive(Debug)]
pub(crate) enum RoutineContinuationOutcome {
    /// The exact authenticated attempt already completed. This classification
    /// is read-only and safe to reuse without another effect.
    Complete(RoutineMediationResult),
    /// The exact authenticated attempt was reserved without a child, staged
    /// launch, or provisioned output and was durably rolled back.
    Reserved { authenticated_head: String },
}

impl RoutineMediationResult {
    pub(crate) fn status(&self) -> RoutineMediatorStatus {
        self.status
    }

    pub(crate) fn request_id(&self) -> Option<&str> {
        self.request_id.as_deref()
    }

    pub(crate) fn protocol_id(&self) -> Option<&str> {
        self.protocol_id.as_deref()
    }

    pub(crate) fn nodes(&self) -> &[RoutineNodeMediation] {
        &self.nodes
    }

    pub(crate) fn recovery_required(&self) -> bool {
        self.recovery_marker.is_some()
    }

    pub(crate) fn recovery_marker(&self) -> Option<&str> {
        self.recovery_marker.as_deref()
    }

    pub(crate) fn continuation(&self) -> Option<&str> {
        self.continuation.as_deref()
    }

    pub(crate) fn attempt_grant(&self) -> Option<&str> {
        self.attempt_grant.as_deref()
    }

    pub(crate) fn checkpoint_head(&self) -> Option<&str> {
        self.checkpoint_head.as_deref()
    }

    pub(crate) fn terminal_outcome(&self) -> Option<RoutineTerminalOutcome> {
        self.terminal_outcome
    }

    pub(crate) fn support_limit(&self) -> &'static str {
        self.support_limit
    }
}

#[derive(Default)]
pub(crate) struct RoutineReuseInput {
    pub(crate) artifacts: Vec<Vec<u8>>,
}

impl RoutineReuseInput {
    pub(crate) fn into_artifacts(self) -> Vec<Vec<u8>> {
        self.artifacts
    }

    pub(crate) fn artifacts(&self) -> &[Vec<u8>] {
        &self.artifacts
    }

    pub(in crate::routine_work::runtime_adapter) fn is_empty(&self) -> bool {
        self.artifacts.is_empty()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct OutputFileRecord {
    pub(crate) sha256: String,
    pub(crate) byte_length: u64,
    pub(crate) unix_mode: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ResultArtifactWire {
    pub(crate) schema_version: String,
    pub(crate) request_id: String,
    pub(crate) protocol_id: String,
    pub(crate) intent_id: String,
    pub(crate) node_id: String,
    pub(crate) behavior_id: String,
    pub(crate) plan_order: usize,
    pub(crate) context_id: String,
    pub(crate) candidate_id: String,
    pub(crate) plan_id: String,
    pub(crate) snapshot_id: String,
    pub(crate) input_id: String,
    pub(crate) tool_identity_sha256: String,
    pub(crate) program_sha256: String,
    pub(crate) environment_sha256: String,
    pub(crate) read_authority_sha256: String,
    pub(crate) dependency_results: BTreeMap<String, String>,
    pub(crate) behavior_sha256: String,
    pub(crate) output_files: BTreeMap<String, OutputFileRecord>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReuseArtifactWire {
    pub(crate) schema_version: String,
    pub(crate) state: String,
    pub(crate) protocol_id: String,
    pub(crate) intent_id: String,
    pub(crate) node_id: String,
    pub(crate) behavior_id: String,
    pub(crate) plan_order: usize,
    pub(crate) context_id: String,
    pub(crate) candidate_id: String,
    pub(crate) plan_id: String,
    pub(crate) snapshot_id: String,
    pub(crate) input_id: String,
    pub(crate) tool_identity_sha256: String,
    pub(crate) program_sha256: String,
    pub(crate) environment_sha256: String,
    pub(crate) read_authority_sha256: String,
    pub(crate) dependency_results: BTreeMap<String, String>,
    pub(crate) output_files: BTreeMap<String, OutputFileRecord>,
    pub(crate) result_artifact: ResultArtifactWire,
    pub(crate) result_artifact_sha256: String,
    pub(crate) mediator_witness_sha256: String,
}

pub(crate) struct ExecutedArtifact {
    pub(crate) node: RoutineNodeMediation,
    pub(crate) result_sha256: String,
    pub(crate) reuse_bytes: Vec<u8>,
}
