use super::model::{MAX_COLLECTION, validate_digest, validate_identifier};
use super::replay::is_active;
use super::{Binding, EffectSink, IntegrationDisposition, OrchestrationError, Orchestrator};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockerClass {
    OrdinaryTechnical,
    MissingDependency,
    MissingTool,
    ExternalAuthority,
    RequiredAccessUnavailable,
    DestructiveDecision,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryAction {
    RetryOrRepair,
    ReplanDependencyClosed,
    RequestExternalAuthority,
    RequestRequiredAccess,
    RequestDestructiveApproval,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Blocker {
    pub blocker_id: String,
    pub node_id: String,
    pub class: BlockerClass,
    pub evidence_digest: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveryDirective {
    pub node_id: String,
    pub action: RecoveryAction,
    pub stop_dependent_work: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveryPlan {
    pub directives: BTreeMap<String, RecoveryDirective>,
    pub advanceable_nodes: Vec<String>,
    pub root_recovery_required: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveryReport {
    pub stale_binding_leases: Vec<String>,
    pub expired_leases: Vec<String>,
    pub orphaned_leases: Vec<String>,
    pub interrupted_root: bool,
    pub resumable_leases: Vec<String>,
    pub ambiguous_operations: Vec<String>,
    pub pending_integration_id: Option<String>,
    pub integration_disposition: Option<IntegrationDisposition>,
}

impl<S: EffectSink> Orchestrator<S> {
    /// Pure recovery analysis. It never calls the injected effect sink.
    pub fn recovery_report(
        &self,
        current_binding: &Binding,
        tick: u64,
        live_workers: &BTreeSet<String>,
    ) -> RecoveryReport {
        let mut report = RecoveryReport {
            interrupted_root: self.projection.root_interrupted,
            ambiguous_operations: self.projection.pending_effects.keys().cloned().collect(),
            pending_integration_id: self
                .projection
                .integration_intent
                .as_ref()
                .and_then(|intent| intent.intent_id().ok()),
            integration_disposition: self
                .projection
                .integration_observation
                .as_ref()
                .map(|observation| observation.disposition),
            ..RecoveryReport::default()
        };
        for (lease_id, runtime) in &self.projection.leases {
            if !is_active(&runtime.phase) {
                continue;
            }
            if &runtime.spec.binding != current_binding {
                report.stale_binding_leases.push(lease_id.clone());
            } else if tick > runtime.deadline_tick {
                report.expired_leases.push(lease_id.clone());
            } else if !live_workers.contains(runtime.spec.owner.as_str()) {
                report.orphaned_leases.push(lease_id.clone());
            } else if !self.projection.root_interrupted {
                report.resumable_leases.push(lease_id.clone());
            }
        }
        report
    }

    /// Pure blocker classification. Stops are scoped to dependent work; legal
    /// independent nodes remain visible unless root recovery is required.
    pub fn recovery_plan(&self, blockers: &[Blocker]) -> Result<RecoveryPlan, OrchestrationError> {
        if blockers.len() > MAX_COLLECTION {
            return Err(OrchestrationError::ResourceLimit);
        }
        let mut directives = BTreeMap::new();
        let mut blocked_nodes = BTreeSet::new();
        for blocker in blockers {
            validate_identifier(&blocker.blocker_id)?;
            validate_identifier(&blocker.node_id)?;
            validate_digest(&blocker.evidence_digest)?;
            if !self.graph.contains(&blocker.node_id) {
                return Err(OrchestrationError::UnknownNode);
            }
            let (action, stop_dependent_work) = match blocker.class {
                BlockerClass::OrdinaryTechnical => (RecoveryAction::RetryOrRepair, false),
                BlockerClass::MissingDependency | BlockerClass::MissingTool => {
                    (RecoveryAction::ReplanDependencyClosed, false)
                }
                BlockerClass::ExternalAuthority => (RecoveryAction::RequestExternalAuthority, true),
                BlockerClass::RequiredAccessUnavailable => {
                    (RecoveryAction::RequestRequiredAccess, true)
                }
                BlockerClass::DestructiveDecision => {
                    (RecoveryAction::RequestDestructiveApproval, true)
                }
            };
            blocked_nodes.insert(blocker.node_id.clone());
            if directives
                .insert(
                    blocker.blocker_id.clone(),
                    RecoveryDirective {
                        node_id: blocker.node_id.clone(),
                        action,
                        stop_dependent_work,
                    },
                )
                .is_some()
            {
                return Err(OrchestrationError::DuplicateOutput);
            }
        }
        let root_recovery_required = self.projection.root_interrupted;
        let advanceable_nodes = if root_recovery_required {
            Vec::new()
        } else {
            self.plan()?
                .ready
                .into_iter()
                .filter(|node| !blocked_nodes.contains(node))
                .collect()
        };
        Ok(RecoveryPlan {
            directives,
            advanceable_nodes,
            root_recovery_required,
        })
    }
}
