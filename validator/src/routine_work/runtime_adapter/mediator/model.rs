use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::routine_work::RepoPath;

use super::DurableAttemptAuthority;

#[cfg(test)]
use super::super::model::RoutineEffectRequest;

#[derive(Clone)]
pub(crate) struct RoutineCancellation {
    cancelled: Arc<AtomicBool>,
}

impl RoutineCancellation {
    pub(crate) fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub(crate) fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub(super) fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

/// Opaque root authority consumed by one routine mediation attempt.
///
/// Production construction is intentionally absent. The canonical root issuer
/// remains root-owned wiring; this module only verifies and consumes grants.
#[must_use = "a root grant must be consumed once or explicitly discarded"]
pub(crate) struct RoutineRootGrant {
    pub(super) grant_id: String,
    pub(super) session_id: String,
    pub(super) request_id: String,
    pub(super) protocol_id: String,
    pub(super) context_id: String,
    pub(super) candidate_id: String,
    pub(super) plan_id: String,
    pub(super) snapshot_id: String,
    pub(super) allowed_output_scopes: Vec<RepoPath>,
    pub(super) recovery_for: Option<String>,
    pub(super) seal: String,
    pub(super) durable: Option<Arc<dyn DurableAttemptAuthority>>,
}

#[cfg(test)]
impl RoutineRootGrant {
    pub(crate) fn test_issue(
        request: &RoutineEffectRequest,
        session_id: impl Into<String>,
        recovery_for: Option<String>,
    ) -> Self {
        let session_id = session_id.into();
        let mut allowed_output_scopes = request
            .intents
            .iter()
            .flat_map(|intent| intent.declared_output_scopes().iter().cloned())
            .collect::<Vec<_>>();
        allowed_output_scopes.sort_by(|left, right| left.as_str().cmp(right.as_str()));
        allowed_output_scopes.dedup();
        let mut grant = Self {
            grant_id: String::new(),
            session_id,
            request_id: request.request_id.clone(),
            protocol_id: request.protocol_id.clone(),
            context_id: request.binding.context_id().to_owned(),
            candidate_id: request.binding.candidate_id().to_owned(),
            plan_id: request.plan_id.clone(),
            snapshot_id: request.snapshot_id.clone(),
            allowed_output_scopes,
            recovery_for,
            seal: String::new(),
            durable: None,
        };
        grant.grant_id = super::grant_identity(&grant).expect("test grant identity");
        grant.seal = super::grant_seal(&grant).expect("test grant seal");
        grant
    }

    pub(crate) fn test_duplicate(&self) -> Self {
        Self {
            grant_id: self.grant_id.clone(),
            session_id: self.session_id.clone(),
            request_id: self.request_id.clone(),
            protocol_id: self.protocol_id.clone(),
            context_id: self.context_id.clone(),
            candidate_id: self.candidate_id.clone(),
            plan_id: self.plan_id.clone(),
            snapshot_id: self.snapshot_id.clone(),
            allowed_output_scopes: self.allowed_output_scopes.clone(),
            recovery_for: self.recovery_for.clone(),
            seal: self.seal.clone(),
            durable: None,
        }
    }

    pub(crate) fn test_recovery_marker(&self) -> String {
        super::recovery_identity(&self.grant_id, &self.protocol_id, &self.request_id)
    }

    pub(crate) fn test_with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = request_id.into();
        self
    }

    pub(crate) fn test_with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = session_id.into();
        self
    }

    pub(crate) fn test_with_context_id(mut self, context_id: impl Into<String>) -> Self {
        self.context_id = context_id.into();
        self
    }

    pub(crate) fn test_with_plan_id(mut self, plan_id: impl Into<String>) -> Self {
        self.plan_id = plan_id.into();
        self
    }

    pub(crate) fn test_with_scopes(mut self, scopes: Vec<RepoPath>) -> Self {
        self.allowed_output_scopes = scopes;
        self
    }

    pub(crate) fn test_with_seal(mut self, seal: impl Into<String>) -> Self {
        self.seal = seal.into();
        self
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum RoutineMediatorStatus {
    CompleteNoOp,
    CompleteExecution,
    IncompleteExecution,
    Cancelled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum RoutineNodeDisposition {
    Executed,
    Reused,
    Failed,
    DependencyFailed,
    Cancelled,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub(crate) struct RoutineNodeMediation {
    pub(super) intent_id: String,
    pub(super) node_id: String,
    pub(super) plan_order: usize,
    pub(super) disposition: RoutineNodeDisposition,
    pub(super) result_artifact_sha256: Option<String>,
    pub(super) failure_code: Option<String>,
}

impl RoutineNodeMediation {
    pub(crate) fn node_id(&self) -> &str {
        &self.node_id
    }

    pub(crate) fn disposition(&self) -> RoutineNodeDisposition {
        self.disposition
    }

    pub(crate) fn result_artifact_sha256(&self) -> Option<&str> {
        self.result_artifact_sha256.as_deref()
    }

    pub(crate) fn failure_code(&self) -> Option<&str> {
        self.failure_code.as_deref()
    }
}

#[derive(Debug)]
#[must_use = "mediation results must be reconciled by their root caller"]
pub(crate) struct RoutineMediationResult {
    pub(super) request_id: Option<String>,
    pub(super) protocol_id: Option<String>,
    pub(super) status: RoutineMediatorStatus,
    pub(super) nodes: Vec<RoutineNodeMediation>,
    pub(super) reuse_artifacts: Vec<Vec<u8>>,
    pub(super) recovery_marker: Option<String>,
    pub(super) support_limit: &'static str,
}

impl RoutineMediationResult {
    pub(crate) fn status(&self) -> RoutineMediatorStatus {
        self.status
    }

    pub(crate) fn request_id(&self) -> Option<&str> {
        self.request_id.as_deref()
    }

    pub(crate) fn nodes(&self) -> &[RoutineNodeMediation] {
        &self.nodes
    }

    pub(crate) fn reuse_artifacts(&self) -> &[Vec<u8>] {
        &self.reuse_artifacts
    }

    pub(crate) fn recovery_marker(&self) -> Option<&str> {
        self.recovery_marker.as_deref()
    }

    pub(crate) fn support_limit(&self) -> &'static str {
        self.support_limit
    }
}

#[derive(Default)]
pub(crate) struct RoutineReuseInput {
    artifacts: Vec<Vec<u8>>,
}

impl RoutineReuseInput {
    pub(crate) fn new(artifacts: Vec<Vec<u8>>) -> Self {
        Self { artifacts }
    }

    pub(super) fn into_artifacts(self) -> Vec<Vec<u8>> {
        self.artifacts
    }

    pub(super) fn artifacts(&self) -> &[Vec<u8>] {
        &self.artifacts
    }

    pub(in crate::routine_work::runtime_adapter) fn is_empty(&self) -> bool {
        self.artifacts.is_empty()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct OutputFileRecord {
    pub(super) sha256: String,
    pub(super) byte_length: u64,
    pub(super) unix_mode: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CommandReport {
    pub(super) schema_version: String,
    pub(super) request_id: String,
    pub(super) protocol_id: String,
    pub(super) intent_id: String,
    pub(super) node_id: String,
    pub(super) outcome: String,
    pub(super) behavior_observed: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ResultArtifactWire {
    pub(super) schema_version: String,
    pub(super) request_id: String,
    pub(super) protocol_id: String,
    pub(super) intent_id: String,
    pub(super) node_id: String,
    pub(super) plan_order: usize,
    pub(super) context_id: String,
    pub(super) candidate_id: String,
    pub(super) plan_id: String,
    pub(super) snapshot_id: String,
    pub(super) input_id: String,
    pub(super) tool_identity_sha256: String,
    pub(super) program_sha256: String,
    pub(super) environment_sha256: String,
    pub(super) read_authority_sha256: String,
    pub(super) dependency_results: BTreeMap<String, String>,
    pub(super) behavior_sha256: String,
    pub(super) output_files: BTreeMap<String, OutputFileRecord>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ReuseArtifactWire {
    pub(super) schema_version: String,
    pub(super) state: String,
    pub(super) protocol_id: String,
    pub(super) intent_id: String,
    pub(super) node_id: String,
    pub(super) plan_order: usize,
    pub(super) context_id: String,
    pub(super) candidate_id: String,
    pub(super) plan_id: String,
    pub(super) snapshot_id: String,
    pub(super) input_id: String,
    pub(super) tool_identity_sha256: String,
    pub(super) program_sha256: String,
    pub(super) environment_sha256: String,
    pub(super) read_authority_sha256: String,
    pub(super) dependency_results: BTreeMap<String, String>,
    pub(super) output_files: BTreeMap<String, OutputFileRecord>,
    pub(super) result_artifact: ResultArtifactWire,
    pub(super) result_artifact_sha256: String,
    pub(super) mediator_witness_sha256: String,
}

pub(super) struct VerifiedReuseArtifact {
    pub(super) wire: ReuseArtifactWire,
    pub(super) canonical_bytes: Vec<u8>,
}

pub(super) struct ExecutedArtifact {
    pub(super) result_sha256: String,
    pub(super) reuse_bytes: Vec<u8>,
}
