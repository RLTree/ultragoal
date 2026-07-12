use super::{PermitTarget, ProductError};
use crate::orchestration::{
    Binding, CanonicalPath, EffectSink, EventKind, JournalHead, Orchestrator, Plan, RecoveryReport,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductCommitment {
    pub worker: String,
    pub result_id: String,
    pub result_commitment_id: String,
    pub artifact_digests: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductSnapshot {
    pub schema_version: String,
    pub binding: Binding,
    pub journal_head: JournalHead,
    pub event_count: usize,
    pub plan: Plan,
    pub recovery: RecoveryReport,
    pub known_workers: BTreeSet<String>,
    pub commitments: BTreeMap<String, ProductCommitment>,
    pub settled_operations: BTreeSet<String>,
}

impl ProductSnapshot {
    pub fn snapshot_id(&self) -> Result<String, ProductError> {
        let bytes = serde_json::to_vec(self).map_err(|_| ProductError::StaleCandidate)?;
        Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
    }

    pub(crate) fn require_target(&self, target: &PermitTarget) -> Result<(), ProductError> {
        if let Some(lease_id) = &target.lease_id {
            let known = self.commitments.contains_key(lease_id)
                || self.recovery.resumable_leases.contains(lease_id)
                || self.recovery.expired_leases.contains(lease_id)
                || self.recovery.orphaned_leases.contains(lease_id)
                || self.recovery.stale_binding_leases.contains(lease_id);
            if !known {
                return Err(ProductError::UnknownWorker);
            }
            if let Some(expected) = &target.result_commitment_id {
                let observed = self
                    .commitments
                    .get(lease_id)
                    .map(|commitment| &commitment.result_commitment_id);
                if observed != Some(expected) {
                    return Err(ProductError::ResultSubstitution);
                }
            }
        } else if target.result_commitment_id.is_some() {
            return Err(ProductError::ResultSubstitution);
        }
        if let Some(operation_id) = &target.operation_id {
            let pending = self
                .recovery
                .ambiguous_operations
                .iter()
                .any(|operation| operation == operation_id);
            if !pending && !self.settled_operations.contains(operation_id) {
                return Err(ProductError::UnknownOperation);
            }
        }
        if target
            .recovered_binding
            .as_ref()
            .is_some_and(|binding| binding != &self.binding)
        {
            return Err(ProductError::StaleCandidate);
        }
        Ok(())
    }
}

pub(crate) fn snapshot<S: EffectSink>(
    engine: &Orchestrator<S>,
    tick: u64,
    live_workers: &BTreeSet<String>,
) -> Result<ProductSnapshot, ProductError> {
    let head = engine
        .journal_head()
        .cloned()
        .ok_or(ProductError::StaleCandidate)?;
    let mut leases = BTreeMap::new();
    let mut commitments = BTreeMap::new();
    let mut settled_operations = BTreeSet::new();
    for event in engine.events() {
        match &event.event {
            EventKind::LeaseGranted { lease } => {
                leases.insert(lease.lease_id.clone(), lease.clone());
            }
            EventKind::WorkerSubmitted {
                lease_id,
                commitment,
                result_commitment_id,
            } => {
                let lease = leases.get(lease_id).ok_or(ProductError::UnknownWorker)?;
                for path in commitment.artifact_digests.keys() {
                    let path = CanonicalPath::parse(path).map_err(ProductError::from)?;
                    let readable = lease.read_paths.iter().any(|root| {
                        path == *root
                            || path
                                .as_str()
                                .strip_prefix(root.as_str())
                                .is_some_and(|suffix| suffix.starts_with('/'))
                    });
                    if !lease.owned_scope.contains_any_path(&path) && !readable {
                        return Err(ProductError::UnknownArtifact);
                    }
                }
                let view = ProductCommitment {
                    worker: lease.owner.as_str().to_owned(),
                    result_id: commitment.result_id.clone(),
                    result_commitment_id: result_commitment_id.clone(),
                    artifact_digests: commitment.artifact_digests.clone(),
                };
                if commitments.insert(lease_id.clone(), view).is_some() {
                    return Err(ProductError::ResultSubstitution);
                }
            }
            EventKind::EffectApplied { receipt, .. } => {
                settled_operations.insert(receipt.operation_id.clone());
            }
            EventKind::EffectReconciled { resolution, .. } => {
                settled_operations.insert(resolution.operation_id.clone());
            }
            _ => {}
        }
    }
    let known_workers = leases
        .values()
        .map(|lease| lease.owner.as_str().to_owned())
        .collect::<BTreeSet<_>>();
    if !live_workers.is_subset(&known_workers) {
        return Err(ProductError::UnknownWorker);
    }
    Ok(ProductSnapshot {
        schema_version: "OrchestrationProductSnapshot-v1".to_owned(),
        binding: engine.binding().clone(),
        journal_head: head,
        event_count: engine.events().len(),
        plan: engine.plan().map_err(ProductError::from)?,
        recovery: engine.recovery_report(engine.binding(), tick, live_workers),
        known_workers,
        commitments,
        settled_operations,
    })
}
