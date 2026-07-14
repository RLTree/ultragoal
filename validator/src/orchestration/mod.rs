//! Adaptive, candidate-bound orchestration primitives.
//!
//! This module proposes work and acceptance to the Ultra root. It never owns
//! shared authority, performs root integration, or raises a product claim.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

mod artifact;
mod effect;
mod effect_transition;
mod engine;
#[path = "authorized_effect.rs"]
mod engine_effect;
#[path = "candidate_integration.rs"]
mod engine_integration;
#[path = "query_runtime.rs"]
mod engine_query;
#[path = "worker_submission.rs"]
mod engine_submission;
mod error;
mod event;
mod graph;
mod integration;
mod integration_transition;
mod journal;
mod lease;
mod model;
pub mod product;
mod reconcile;
mod recovery;
mod replay;
mod scope;
mod scope_policy;
mod transition;
mod worker;
mod workspace;

pub use artifact::{ArtifactRecord, ArtifactWorkspace, VerifiedArtifactSet};
pub use effect::{EffectOutcome, EffectReceipt, EffectRequest, EffectResolution, EffectSink};
pub use engine::Orchestrator;
pub use error::OrchestrationError;
pub use event::{EventKind, EventLog, OrchestrationEvent, ResultCommitment, ReviewDecision};
pub use graph::{Plan, PlanBlock, WorkGraph, WorkProgress};
pub use integration::{IntegrationDisposition, RootIntegrationIntent, RootIntegrationObservation};
pub(crate) use journal::{
    encode_log as encode_orchestration_log, head_for as orchestration_head_for,
};
pub use journal::{FileJournal, JournalHead, JournalSnapshot};
pub use lease::{LeaseRegistry, LeaseSpec, PrerequisiteEvidence};
pub use model::{
    Actor, Binding, BootstrapEvidence, EffectClass, EffectGrant, Principal, SafetyClass,
    WorkPackage,
};
pub use reconcile::{
    propose_acceptance, AcceptanceProposal, AcceptedLeaseIntegration, ReviewRecord,
    RootIntegrationReceipt,
};
pub use recovery::{
    Blocker, BlockerClass, RecoveryAction, RecoveryDirective, RecoveryPlan, RecoveryReport,
};
pub use scope::{CanonicalPath, OwnedScope};
pub use scope_policy::ScopePolicy;
pub use worker::{EffectUse, WorkerResultV1};
pub use workspace::RootWorkspace;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RootChangeRequest {
    pub path: String,
    pub expected_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request: Option<String>,
}

impl RootChangeRequest {
    pub fn new(
        path: &str,
        expected_sha256: &str,
        request: Option<&str>,
    ) -> Result<Self, OrchestrationError> {
        let value = Self {
            path: path.to_owned(),
            expected_sha256: expected_sha256.to_owned(),
            request: request.map(str::to_owned),
        };
        value.validate()?;
        Ok(value)
    }

    fn validate(&self) -> Result<(), OrchestrationError> {
        CanonicalPath::parse(&self.path)?;
        model::validate_digest(&self.expected_sha256)?;
        if self.request.as_ref().is_some_and(|text| {
            text.is_empty() || text.len() > 4096 || text.chars().any(char::is_control)
        }) {
            return Err(OrchestrationError::InvalidWorkerResult);
        }
        Ok(())
    }
}

pub(crate) fn root_change_map(
    requests: &[RootChangeRequest],
) -> Result<BTreeMap<String, String>, OrchestrationError> {
    if requests.len() > model::MAX_COLLECTION {
        return Err(OrchestrationError::ResourceLimit);
    }
    let map = requests
        .iter()
        .map(|request| {
            request.validate()?;
            Ok((request.path.clone(), request.expected_sha256.clone()))
        })
        .collect::<Result<BTreeMap<_, _>, OrchestrationError>>()?;
    if map.len() != requests.len() {
        return Err(OrchestrationError::DuplicateOutput);
    }
    validate_root_change_map(&map)?;
    Ok(map)
}

pub(crate) fn validate_root_change_map(
    map: &BTreeMap<String, String>,
) -> Result<(), OrchestrationError> {
    if map.len() > model::MAX_COLLECTION {
        return Err(OrchestrationError::ResourceLimit);
    }
    let mut paths = Vec::with_capacity(map.len());
    for (raw, digest) in map {
        let path = CanonicalPath::parse(raw)?;
        model::validate_digest(digest)?;
        if paths
            .iter()
            .any(|prior: &CanonicalPath| prior.overlaps(&path))
        {
            return Err(OrchestrationError::DuplicateOutput);
        }
        paths.push(path);
    }
    Ok(())
}

pub(crate) fn root_change_digest(
    map: &BTreeMap<String, String>,
) -> Result<String, OrchestrationError> {
    validate_root_change_map(map)?;
    let bytes = serde_json::to_vec(map).map_err(|_| OrchestrationError::InvalidWorkerResult)?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

pub(crate) fn result_commitment_from_parts(
    binding: &Binding,
    node_id: &str,
    lease_id: &str,
    result: &WorkerResultV1,
) -> Result<event::ResultCommitment, OrchestrationError> {
    let artifact_digests = result
        .artifacts
        .iter()
        .map(|artifact| (artifact.path.clone(), artifact.sha256.clone()))
        .collect::<BTreeMap<_, _>>();
    if artifact_digests.len() != result.artifacts.len() {
        return Err(OrchestrationError::DuplicateOutput);
    }
    let expected_root_changes = root_change_map(&result.requested_root_changes)?;
    let commitment = event::ResultCommitment {
        binding: binding.clone(),
        node_id: node_id.to_owned(),
        lease_id: lease_id.to_owned(),
        result_id: result.result_id()?,
        artifact_digests,
        requested_root_change_count: expected_root_changes.len(),
        requested_root_changes_digest: root_change_digest(&expected_root_changes)?,
        expected_root_changes,
    };
    commitment.validate()?;
    Ok(commitment)
}
