use super::evidence::{Actor, ActorRole};
use super::false_pass::{MODEL_METHOD, MODEL_OBSERVATION_METHOD};
use super::false_pass_receipt::SemanticControlModel;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub(super) fn validate_model_shape(model: &SemanticControlModel) -> Result<(), String> {
    if model.model_id.is_empty()
        || !raw_digest(&model.registry_digest)
        || model.claim_id.is_empty()
        || model.control_id.is_empty()
        || !digest(&model.control_definition_digest)
        || !digest(&model.model_spec_digest)
        || !digest(&model.authority_nonce)
        || !digest(&model.negative_stimulus_digest)
        || model.expected_failure_contract.is_empty()
        || !model.modeler.actor_id.starts_with("claim-control-modeler:")
        || model.modeler.roles != BTreeSet::from([ActorRole::SemanticModeler])
        || !model.model_method.starts_with(&format!("{MODEL_METHOD}:"))
        || !digest(&model.model_implementation_digest)
        || model.modeled_at_unix_ms == 0
        || model.max_age_ms == 0
        || !digest(&model.live_context_id)
        || !digest(&model.candidate_id)
        || model.truth_surface.is_empty()
        || model.declared_ceiling.is_empty()
        || !digest(&model.modeled_result_digest)
        || !digest(&model.model_record_digest)
        || model_record_digest(model).as_deref() != Ok(model.model_record_digest.as_str())
    {
        return Err("claims-semantic-control-model-incomplete".to_owned());
    }
    Ok(())
}

pub(super) fn model_record_digest(model: &SemanticControlModel) -> Result<String, String> {
    let bytes = serde_json::to_vec(&(
        (
            &model.model_id,
            &model.registry_digest,
            &model.claim_id,
            &model.control_id,
            &model.control_definition_digest,
            &model.model_spec_digest,
            &model.authority_nonce,
            &model.negative_stimulus_digest,
            &model.expected_failure_contract,
            &model.modeler,
        ),
        (
            &model.model_method,
            &model.model_implementation_digest,
            model.modeled_at_unix_ms,
            &model.live_context_id,
            &model.candidate_id,
            model.max_age_ms,
            &model.truth_surface,
            &model.declared_ceiling,
            &model.modeled_result_digest,
        ),
    ))
    .map_err(|_| "claims-semantic-control-model-encode-failed".to_owned())?;
    Ok(digest_bytes(&bytes))
}

pub(super) fn seal_digest(
    model: &SemanticControlModel,
    observer: &Actor,
    observation_method: &str,
    observed_at_unix_ms: u64,
    observed_model_digest: &str,
) -> Result<String, String> {
    if !observer
        .actor_id
        .starts_with("claim-control-model-observer:")
        || observer.roles
            != BTreeSet::from([
                ActorRole::IndependentObserver,
                ActorRole::SemanticModelObserver,
            ])
        || !observation_method.starts_with(&format!("{MODEL_OBSERVATION_METHOD}:"))
        || observed_at_unix_ms == 0
        || !digest(observed_model_digest)
        || observed_model_digest != model.model_record_digest
    {
        return Err("claims-semantic-control-observer-incomplete".to_owned());
    }
    let bytes = serde_json::to_vec(&(
        model,
        observer,
        observation_method,
        observed_at_unix_ms,
        observed_model_digest,
    ))
    .map_err(|_| "claims-semantic-control-observation-encode-failed".to_owned())?;
    Ok(digest_bytes(&bytes))
}

fn digest(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|hex| {
        hex.len() == 64
            && hex
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}

fn raw_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn digest_bytes(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
