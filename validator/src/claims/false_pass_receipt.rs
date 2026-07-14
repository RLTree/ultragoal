use super::evidence::Actor;
use super::false_pass::ModelAuthority;
use super::false_pass_integrity::{model_record_digest, seal_digest, validate_model_shape};
use super::semantic_control_model_draft::SemanticControlModelDraft;
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SemanticControlModel {
    pub(super) model_id: String,
    pub(super) registry_digest: String,
    pub(super) claim_id: String,
    pub(super) control_id: String,
    pub(super) control_definition_digest: String,
    pub(super) model_spec_digest: String,
    pub(super) authority_nonce: String,
    pub(super) negative_stimulus_digest: String,
    pub(super) expected_failure_contract: String,
    pub(super) modeler: Actor,
    pub(super) model_method: String,
    pub(super) model_implementation_digest: String,
    pub(super) modeled_at_unix_ms: u64,
    pub(super) live_context_id: String,
    pub(super) candidate_id: String,
    pub(super) max_age_ms: u64,
    pub(super) truth_surface: String,
    pub(super) declared_ceiling: String,
    pub(super) modeled_result_digest: String,
    pub(super) model_record_digest: String,
}

impl SemanticControlModel {
    pub(super) fn from_authority(
        _authority: &ModelAuthority,
        draft: SemanticControlModelDraft,
    ) -> Result<Self, String> {
        let mut model = Self {
            model_id: draft.model_id,
            registry_digest: draft.registry_digest,
            claim_id: draft.claim_id,
            control_id: draft.control_id,
            control_definition_digest: draft.control_definition_digest,
            model_spec_digest: draft.model_spec_digest,
            authority_nonce: draft.authority_nonce,
            negative_stimulus_digest: draft.negative_stimulus_digest,
            expected_failure_contract: draft.expected_failure_contract,
            modeler: draft.modeler,
            model_method: draft.model_method,
            model_implementation_digest: draft.model_implementation_digest,
            modeled_at_unix_ms: draft.modeled_at_unix_ms,
            live_context_id: draft.live_context_id,
            candidate_id: draft.candidate_id,
            max_age_ms: draft.max_age_ms,
            truth_surface: draft.truth_surface,
            declared_ceiling: draft.declared_ceiling,
            modeled_result_digest: draft.modeled_result_digest,
            model_record_digest: String::new(),
        };
        model.model_record_digest = model_record_digest(&model)?;
        Ok(model)
    }

    pub fn model_id(&self) -> &str {
        &self.model_id
    }
    pub fn registry_digest(&self) -> &str {
        &self.registry_digest
    }
    pub fn claim_id(&self) -> &str {
        &self.claim_id
    }
    pub fn control_id(&self) -> &str {
        &self.control_id
    }
    pub fn control_definition_digest(&self) -> &str {
        &self.control_definition_digest
    }
    pub fn model_spec_digest(&self) -> &str {
        &self.model_spec_digest
    }
    pub fn authority_nonce(&self) -> &str {
        &self.authority_nonce
    }
    pub fn negative_stimulus_digest(&self) -> &str {
        &self.negative_stimulus_digest
    }
    pub fn expected_failure_contract(&self) -> &str {
        &self.expected_failure_contract
    }
    pub fn modeler(&self) -> &Actor {
        &self.modeler
    }
    pub fn model_method(&self) -> &str {
        &self.model_method
    }
    pub fn model_implementation_digest(&self) -> &str {
        &self.model_implementation_digest
    }
    pub fn modeled_at_unix_ms(&self) -> u64 {
        self.modeled_at_unix_ms
    }
    pub fn live_context_id(&self) -> &str {
        &self.live_context_id
    }
    pub fn candidate_id(&self) -> &str {
        &self.candidate_id
    }
    pub fn max_age_ms(&self) -> u64 {
        self.max_age_ms
    }
    pub fn truth_surface(&self) -> &str {
        &self.truth_surface
    }
    pub fn declared_ceiling(&self) -> &str {
        &self.declared_ceiling
    }
    pub fn modeled_result_digest(&self) -> &str {
        &self.modeled_result_digest
    }
    pub fn model_record_digest(&self) -> &str {
        &self.model_record_digest
    }
}

impl<'de> Deserialize<'de> for SemanticControlModel {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Err(serde::de::Error::custom(
            "claims-semantic-control-model-deserialization-prohibited",
        ))
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SemanticControlObservation {
    model: SemanticControlModel,
    observer: Actor,
    observation_method: String,
    observed_at_unix_ms: u64,
    observed_model_digest: String,
    observed_digest: String,
}

impl SemanticControlObservation {
    pub(super) fn from_authority(
        _authority: &ModelAuthority,
        model: SemanticControlModel,
        observer: Actor,
        observation_method: String,
        observed_at_unix_ms: u64,
    ) -> Result<Self, String> {
        let observed_model_digest = model.model_record_digest.clone();
        let observed_digest = seal_digest(
            &model,
            &observer,
            &observation_method,
            observed_at_unix_ms,
            &observed_model_digest,
        )?;
        Ok(Self {
            model,
            observer,
            observation_method,
            observed_at_unix_ms,
            observed_model_digest,
            observed_digest,
        })
    }

    pub fn model(&self) -> &SemanticControlModel {
        &self.model
    }
    pub fn observer(&self) -> &Actor {
        &self.observer
    }
    pub fn observation_method(&self) -> &str {
        &self.observation_method
    }
    pub fn observed_at_unix_ms(&self) -> u64 {
        self.observed_at_unix_ms
    }
    pub fn observed_digest(&self) -> &str {
        &self.observed_digest
    }

    pub fn verify_integrity(&self) -> Result<(), String> {
        validate_model_shape(&self.model)?;
        let actual = seal_digest(
            &self.model,
            &self.observer,
            &self.observation_method,
            self.observed_at_unix_ms,
            &self.observed_model_digest,
        )?;
        if actual == self.observed_digest {
            Ok(())
        } else {
            Err("claims-semantic-control-observation-mutated".to_owned())
        }
    }
}

impl<'de> Deserialize<'de> for SemanticControlObservation {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Err(serde::de::Error::custom(
            "claims-semantic-control-observation-deserialization-prohibited",
        ))
    }
}
