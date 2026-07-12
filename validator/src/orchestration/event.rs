use super::effect::{EffectReceipt, EffectRequest, EffectResolution};
use super::model::{
    MAX_COLLECTION, validate_actor_identifier, validate_digest, validate_identifier,
};
use super::{
    AcceptanceProposal, Actor, Binding, BootstrapEvidence, CanonicalPath, LeaseSpec,
    OrchestrationError, ReviewRecord, RootIntegrationIntent, RootIntegrationObservation,
    RootIntegrationReceipt,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

const MAX_EVENTS: usize = 16_384;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewDecision {
    Pass,
    Rework,
    Reject,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResultCommitment {
    pub binding: Binding,
    pub node_id: String,
    pub lease_id: String,
    pub result_id: String,
    pub artifact_digests: BTreeMap<String, String>,
    pub expected_root_changes: BTreeMap<String, String>,
    pub requested_root_change_count: usize,
    pub requested_root_changes_digest: String,
}

impl ResultCommitment {
    pub(crate) fn commitment_id(&self) -> Result<String, OrchestrationError> {
        self.validate()?;
        let bytes =
            serde_json::to_vec(self).map_err(|_| OrchestrationError::InvalidWorkerResult)?;
        Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
    }

    pub(crate) fn validate(&self) -> Result<(), OrchestrationError> {
        self.binding.validate()?;
        validate_identifier(&self.node_id)?;
        validate_identifier(&self.lease_id)?;
        validate_digest(&self.result_id)?;
        super::validate_root_change_map(&self.expected_root_changes)?;
        let expected_digest = super::root_change_digest(&self.expected_root_changes)?;
        if self.artifact_digests.len() > MAX_COLLECTION
            || self.requested_root_change_count != self.expected_root_changes.len()
            || self.requested_root_changes_digest != expected_digest
        {
            return Err(OrchestrationError::InvalidWorkerResult);
        }
        for (path, digest) in &self.artifact_digests {
            CanonicalPath::parse(path)?;
            validate_digest(digest)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EventKind {
    Bootstrapped {
        evidence: BootstrapEvidence,
    },
    LeaseGranted {
        lease: LeaseSpec,
    },
    WorkStarted {
        lease_id: String,
    },
    Heartbeat {
        lease_id: String,
    },
    WorkerSubmitted {
        lease_id: String,
        commitment: ResultCommitment,
        result_commitment_id: String,
    },
    ReviewAssigned {
        lease_id: String,
        reviewer: Actor,
    },
    ReviewRecorded {
        lease_id: String,
        review: ReviewRecord,
        result_commitment_id: String,
    },
    RetryScheduled {
        lease_id: String,
        new_deadline_tick: u64,
    },
    LeaseCancelled {
        lease_id: String,
    },
    RootInterrupted,
    RootRecovered,
    RootAccepted {
        proposal: AcceptanceProposal,
    },
    RootIntegrationStarted {
        intent: RootIntegrationIntent,
    },
    RootIntegrationObserved {
        observation: RootIntegrationObservation,
    },
    CandidateRebound {
        observation: RootIntegrationObservation,
        integration: RootIntegrationReceipt,
    },
    EffectIntent {
        lease_id: String,
        request: EffectRequest,
    },
    EffectApplied {
        lease_id: String,
        receipt: EffectReceipt,
    },
    EffectReconciled {
        lease_id: String,
        resolution: EffectResolution,
    },
}

impl EventKind {
    pub(crate) fn lease_id(&self) -> Option<&str> {
        match self {
            Self::WorkStarted { lease_id }
            | Self::Heartbeat { lease_id }
            | Self::WorkerSubmitted { lease_id, .. }
            | Self::ReviewAssigned { lease_id, .. }
            | Self::ReviewRecorded { lease_id, .. }
            | Self::RetryScheduled { lease_id, .. }
            | Self::LeaseCancelled { lease_id }
            | Self::EffectIntent { lease_id, .. }
            | Self::EffectApplied { lease_id, .. }
            | Self::EffectReconciled { lease_id, .. } => Some(lease_id),
            Self::RootAccepted { proposal } => Some(&proposal.lease_id),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrchestrationEvent {
    pub sequence: u64,
    pub prior_event_id: Option<String>,
    pub event_id: String,
    pub binding: Binding,
    pub actor: Actor,
    pub logical_tick: u64,
    pub event: EventKind,
}

#[derive(Serialize)]
struct EventEnvelope<'a> {
    sequence: u64,
    prior_event_id: &'a Option<String>,
    binding: &'a Binding,
    actor: &'a Actor,
    logical_tick: u64,
    event: &'a EventKind,
}

impl OrchestrationEvent {
    pub fn create(
        sequence: u64,
        prior_event_id: Option<String>,
        binding: Binding,
        actor: Actor,
        logical_tick: u64,
        event: EventKind,
    ) -> Result<Self, OrchestrationError> {
        binding.validate()?;
        validate_actor_identifier(actor.as_str())?;
        if let Some(prior) = &prior_event_id {
            validate_digest(prior)?;
        }
        let envelope = EventEnvelope {
            sequence,
            prior_event_id: &prior_event_id,
            binding: &binding,
            actor: &actor,
            logical_tick,
            event: &event,
        };
        let bytes = serde_json::to_vec(&envelope).map_err(|_| OrchestrationError::InvalidEvent)?;
        let event_id = format!("sha256:{:x}", Sha256::digest(bytes));
        Ok(Self {
            sequence,
            prior_event_id,
            event_id,
            binding,
            actor,
            logical_tick,
            event,
        })
    }

    pub(crate) fn verify_identity(&self) -> Result<(), OrchestrationError> {
        let rebuilt = Self::create(
            self.sequence,
            self.prior_event_id.clone(),
            self.binding.clone(),
            self.actor.clone(),
            self.logical_tick,
            self.event.clone(),
        )?;
        if rebuilt.event_id != self.event_id {
            return Err(OrchestrationError::InvalidEvent);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EventLog(pub Vec<OrchestrationEvent>);

impl EventLog {
    pub fn events(&self) -> &[OrchestrationEvent] {
        &self.0
    }

    pub(crate) fn next(
        &self,
        binding: &Binding,
        actor: Actor,
        tick: u64,
        event: EventKind,
    ) -> Result<OrchestrationEvent, OrchestrationError> {
        if self.0.len() >= MAX_EVENTS {
            return Err(OrchestrationError::ResourceLimit);
        }
        OrchestrationEvent::create(
            self.0.len() as u64,
            self.0.last().map(|prior| prior.event_id.clone()),
            binding.clone(),
            actor,
            tick,
            event,
        )
    }

    pub(crate) fn ensure_capacity(&self) -> Result<(), OrchestrationError> {
        if self.0.len() >= MAX_EVENTS {
            return Err(OrchestrationError::ResourceLimit);
        }
        Ok(())
    }
}
