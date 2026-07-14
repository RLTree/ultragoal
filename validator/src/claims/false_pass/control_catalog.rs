use super::*;

impl NamedControlDefinition {
    pub(crate) fn derive(
        registry_digest: &str,
        definition: &ClaimDefinition,
        control_id: &str,
        control_ordinal: usize,
    ) -> Result<Self, String> {
        if registry_digest != ADOPTED_CLAIM_REGISTRY_SHA256
            || definition
                .false_pass_controls
                .get(control_ordinal)
                .map(String::as_str)
                != Some(control_id)
        {
            return Err("claims-control-definition-not-adopted".to_owned());
        }
        let base = serde_json::to_vec(&(
            CONTROL_CATALOG_VERSION,
            registry_digest,
            &definition.claim_id,
            control_id,
            control_ordinal,
            &definition.truth_surface,
            &definition.allowed_ceiling_on_pass,
        ))
        .map_err(|_| "claims-control-definition-encode-failed".to_owned())?;
        let definition_digest = digest_bytes(&base);
        let short = short_digest(&definition_digest);
        let claim_slug = definition
            .claim_id
            .trim_start_matches("CL-")
            .to_ascii_lowercase();
        Ok(Self {
            registry_digest: registry_digest.to_owned(),
            claim_id: definition.claim_id.clone(),
            control_id: control_id.to_owned(),
            control_ordinal,
            truth_surface: definition.truth_surface.clone(),
            declared_ceiling: definition.allowed_ceiling_on_pass.clone(),
            expected_failure_contract: format!(
                "claims-negative-control-rejected:{claim_slug}:{control_ordinal}:{short}"
            ),
            definition_digest,
        })
    }
}

pub(crate) fn expected_control_definition(
    registry_digest: &str,
    definition: &ClaimDefinition,
    control_id: &str,
) -> Result<NamedControlDefinition, String> {
    let ordinal = definition
        .false_pass_controls
        .iter()
        .position(|candidate| candidate == control_id)
        .ok_or_else(|| "claims-control-not-required-by-claim".to_owned())?;
    NamedControlDefinition::derive(registry_digest, definition, control_id, ordinal)
}

pub(crate) fn validate_named_control_model(
    registry_digest: &str,
    definition: &ClaimDefinition,
    model: &SemanticControlModel,
) -> Result<(), String> {
    let expected = expected_control_definition(registry_digest, definition, model.control_id())?;
    let plan = SemanticControlPlan::issue(
        &expected,
        model.live_context_id(),
        model.candidate_id(),
        model.authority_nonce(),
    )?;
    let expected_model_id = digest_value(&format!(
        "claim-control-semantic-model-id-v1:{}:{}:{}",
        plan.model_spec_digest,
        model.authority_nonce(),
        plan.modeled_result_digest
    ));
    if model.registry_digest() != registry_digest
        || model.claim_id() != definition.claim_id
        || model.control_id() != expected.control_id
        || model.control_definition_digest() != expected.definition_digest
        || model.model_spec_digest() != plan.model_spec_digest
        || model.negative_stimulus_digest() != plan.negative_stimulus_digest
        || model.expected_failure_contract() != expected.expected_failure_contract
        || model.model_implementation_digest() != plan.model_implementation_digest
        || model.modeler().actor_id != expected_modeler_id(&expected)
        || model.truth_surface() != expected.truth_surface
        || model.declared_ceiling() != expected.declared_ceiling
        || model.max_age_ms() != MODEL_MAX_AGE_MS
        || model.modeled_result_digest() != plan.modeled_result_digest
        || model.model_id() != expected_model_id
        || method_sequence(model.model_method(), MODEL_METHOD).is_none()
    {
        return Err("claims-control-semantic-model-definition-mismatch".to_owned());
    }
    Ok(())
}

pub(crate) fn expected_model_observer_id(
    registry_digest: &str,
    definition: &ClaimDefinition,
    control_id: &str,
) -> Result<String, String> {
    let expected = expected_control_definition(registry_digest, definition, control_id)?;
    Ok(model_observer_id(&expected))
}

pub(crate) fn expected_modeler_id(control: &NamedControlDefinition) -> String {
    format!(
        "claim-control-modeler:{}",
        short_digest(&control.definition_digest)
    )
}

pub(crate) fn model_observer_id(control: &NamedControlDefinition) -> String {
    format!(
        "claim-control-model-observer:{}",
        short_digest(&control.definition_digest)
    )
}

pub(crate) fn method_sequence(method: &str, prefix: &str) -> Option<u64> {
    method
        .strip_prefix(prefix)
        .and_then(|suffix| suffix.strip_prefix(':'))
        .and_then(|suffix| suffix.parse::<u64>().ok())
        .filter(|value| *value > 0)
}

pub(crate) fn validate_registry_control_index(
    definitions: &ClaimDefinitions,
) -> Result<(), String> {
    let mut control_bindings = BTreeSet::new();
    for claim_id in definitions.order() {
        let definition = definitions
            .definition(claim_id)
            .ok_or_else(|| "claims-control-catalog-unknown-claim".to_owned())?;
        for control_id in &definition.false_pass_controls {
            if !control_bindings.insert((claim_id.clone(), control_id.clone())) {
                return Err("claims-control-catalog-conflict".to_owned());
            }
        }
    }
    let expected = definitions
        .order()
        .iter()
        .map(|claim_id| {
            definitions
                .definition(claim_id)
                .map(|definition| definition.false_pass_controls.len())
                .unwrap_or(0)
        })
        .sum::<usize>();
    if control_bindings.len() != expected {
        return Err("claims-control-catalog-incomplete".to_owned());
    }
    Ok(())
}

pub(crate) struct SemanticControlPlan {
    pub(crate) model_spec_digest: String,
    pub(crate) negative_stimulus_digest: String,
    pub(crate) model_implementation_digest: String,
    pub(crate) modeled_result_digest: String,
}
