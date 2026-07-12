use super::false_pass::ExecutionObservation;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActorRole {
    EvidenceProducer,
    IndependentObserver,
    ControlExecutor,
    ExecutionObserver,
    MaterialScorer,
    IndependentReviewer,
    RootAuthority,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Actor {
    pub actor_id: String,
    pub roles: BTreeSet<ActorRole>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObligationKind {
    RequiredEvidence,
    RequiredSurface,
    RequiredTool,
    RequiredDecision,
    FalsePassControl,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct ClaimObligation {
    pub kind: ObligationKind,
    pub id: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ObligationResult {
    Supported {
        result_digest: String,
    },
    ObservedFailure {
        result_digest: String,
        failure_code: String,
    },
    Contradicted {
        result_digest: String,
        reason: String,
    },
}

impl ObligationResult {
    pub fn result_digest(&self) -> &str {
        match self {
            Self::Supported { result_digest }
            | Self::ObservedFailure { result_digest, .. }
            | Self::Contradicted { result_digest, .. } => result_digest,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    DirectObservation,
    Receipt,
    TestOutput,
    Telemetry,
    Manifest,
    WorkerSummary,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvidenceEnvelope {
    pub evidence_id: String,
    pub claim_id: String,
    pub obligation: ClaimObligation,
    pub producer: Actor,
    pub method: String,
    pub live_context_id: String,
    pub candidate_id: String,
    pub observed_at_unix_ms: u64,
    pub max_age_ms: u64,
    pub truth_surface: String,
    pub declared_ceiling: String,
    pub kind: EvidenceKind,
    pub result: ObligationResult,
    #[serde(skip_deserializing, default)]
    pub false_pass_execution: Option<ExecutionObservation>,
    pub inputs: BTreeMap<String, String>,
    pub environment_and_tools: BTreeMap<String, String>,
    pub effects: BTreeMap<String, String>,
    pub outputs: BTreeMap<String, String>,
    pub artifact_digests: BTreeSet<String>,
    pub limitations: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Observation {
    envelope: EvidenceEnvelope,
    observed_digest: String,
}

impl EvidenceEnvelope {
    pub fn observe(self) -> Result<Observation, String> {
        validate_envelope_shape(&self)?;
        let bytes = serde_json::to_vec(&self).map_err(|_| "claims-evidence-encode-failed")?;
        Ok(Observation {
            envelope: self,
            observed_digest: format!("sha256:{:x}", Sha256::digest(bytes)),
        })
    }
}

impl Observation {
    pub fn envelope(&self) -> &EvidenceEnvelope {
        &self.envelope
    }
    pub fn evidence_id(&self) -> &str {
        &self.envelope.evidence_id
    }
    pub fn observed_digest(&self) -> &str {
        &self.observed_digest
    }
    pub fn verify_integrity(&self) -> Result<(), String> {
        let bytes = serde_json::to_vec(&self.envelope)
            .map_err(|_| "claims-evidence-encode-failed".to_owned())?;
        let actual = format!("sha256:{:x}", Sha256::digest(bytes));
        if actual == self.observed_digest {
            Ok(())
        } else {
            Err("claims-observation-mutated".to_owned())
        }
    }

    pub(crate) fn validate(&self) -> Result<(), String> {
        validate_envelope_shape(&self.envelope)?;
        self.verify_integrity()
    }
}

fn validate_envelope_shape(envelope: &EvidenceEnvelope) -> Result<(), String> {
    let required_text = [
        &envelope.evidence_id,
        &envelope.claim_id,
        &envelope.obligation.id,
        &envelope.producer.actor_id,
        &envelope.method,
        &envelope.live_context_id,
        &envelope.candidate_id,
        &envelope.truth_surface,
        &envelope.declared_ceiling,
    ];
    if required_text.iter().any(|value| value.is_empty())
        || envelope.producer.roles.is_empty()
        || envelope.max_age_ms == 0
        || envelope.limitations.is_empty()
        || envelope.limitations.iter().any(|value| value.is_empty())
        || !digest_map(&envelope.inputs)
        || !digest_map(&envelope.environment_and_tools)
        || !digest_map(&envelope.outputs)
        || envelope.effects.is_empty()
        || envelope
            .effects
            .iter()
            .any(|(key, value)| key.is_empty() || value.is_empty())
        || envelope.artifact_digests.is_empty()
        || envelope.artifact_digests.iter().any(|value| !digest(value))
        || !digest(envelope.result.result_digest())
        || !envelope
            .outputs
            .values()
            .any(|value| value == envelope.result.result_digest())
    {
        return Err("claims-evidence-envelope-incomplete".to_owned());
    }
    if let Some(observation) = &envelope.false_pass_execution {
        observation.verify_integrity()?;
    }
    match &envelope.result {
        ObligationResult::ObservedFailure { failure_code, .. } if failure_code.is_empty() => {
            Err("claims-evidence-failure-unobserved".to_owned())
        }
        ObligationResult::Contradicted { reason, .. } if reason.is_empty() => {
            Err("claims-evidence-contradiction-unexplained".to_owned())
        }
        _ => Ok(()),
    }
}

fn digest_map(values: &BTreeMap<String, String>) -> bool {
    !values.is_empty()
        && values
            .iter()
            .all(|(key, value)| !key.is_empty() && digest(value))
}

fn digest(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|hex| {
        hex.len() == 64
            && hex
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}
