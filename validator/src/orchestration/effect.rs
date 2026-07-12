use super::model::{validate_digest, validate_identifier};
use super::{Binding, EffectGrant, OrchestrationError};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const EFFECT_RESOLUTION_SCHEMA: &str = "OrchestrationEffectResolution-v1";
const EFFECT_RESOLUTION_DOMAIN: &[u8] = b"harness-ultragoal/orchestration-effect-resolution/v1\0";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EffectRequest {
    pub lease_id: String,
    pub binding: Binding,
    pub effect: EffectGrant,
    pub operation_id: String,
    pub payload_digest: String,
}

impl EffectRequest {
    pub(crate) fn validate(&self) -> Result<(), OrchestrationError> {
        validate_identifier(&self.lease_id)?;
        self.binding.validate()?;
        self.effect.validate()?;
        validate_identifier(&self.operation_id)?;
        validate_digest(&self.payload_digest)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EffectReceipt {
    pub operation_id: String,
    pub effect: EffectGrant,
    pub receipt_digest: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case", deny_unknown_fields)]
pub enum EffectOutcome {
    Applied { receipt: EffectReceipt },
    NotApplied,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EffectResolution {
    pub operation_id: String,
    pub evidence_digest: String,
    pub outcome: EffectOutcome,
}

impl EffectResolution {
    pub(crate) fn validate_shape(&self) -> Result<(), OrchestrationError> {
        validate_identifier(&self.operation_id)?;
        validate_digest(&self.evidence_digest)?;
        if let EffectOutcome::Applied { receipt } = &self.outcome {
            receipt.validate()?;
        }
        Ok(())
    }

    pub(crate) fn commitment_id(&self) -> Result<String, OrchestrationError> {
        #[derive(Serialize)]
        struct Commitment<'a> {
            schema_version: &'a str,
            resolution: &'a EffectResolution,
        }

        self.validate_shape()?;
        let bytes = serde_json::to_vec(&Commitment {
            schema_version: EFFECT_RESOLUTION_SCHEMA,
            resolution: self,
        })
        .map_err(|_| OrchestrationError::InvalidEvent)?;
        let mut hasher = Sha256::new();
        hasher.update(EFFECT_RESOLUTION_DOMAIN);
        hasher.update((bytes.len() as u64).to_be_bytes());
        hasher.update(bytes);
        Ok(format!("sha256:{:x}", hasher.finalize()))
    }

    pub(crate) fn validate_for(&self, request: &EffectRequest) -> Result<(), OrchestrationError> {
        self.validate_shape()?;
        if self.operation_id != request.operation_id {
            return Err(OrchestrationError::EffectDenied);
        }
        if let EffectOutcome::Applied { receipt } = &self.outcome {
            receipt.validate_for(request)?;
        }
        Ok(())
    }
}

impl EffectReceipt {
    pub(crate) fn validate(&self) -> Result<(), OrchestrationError> {
        validate_identifier(&self.operation_id)?;
        self.effect.validate()?;
        validate_digest(&self.receipt_digest)
    }

    pub(crate) fn validate_for(&self, request: &EffectRequest) -> Result<(), OrchestrationError> {
        if self.operation_id != request.operation_id || self.effect != request.effect {
            return Err(OrchestrationError::EffectDenied);
        }
        self.validate()
    }
}

/// The only boundary through which the kernel can request a local or external
/// mutation. An intent is durably recorded before this capability is invoked.
/// Any error or incongruent receipt is therefore treated as ambiguous until
/// the root supplies independent applied/not-applied evidence. Inspect, plan,
/// replay, and recovery never receive this capability.
pub trait EffectSink {
    fn apply(&mut self, request: &EffectRequest) -> Result<EffectReceipt, OrchestrationError>;
}
