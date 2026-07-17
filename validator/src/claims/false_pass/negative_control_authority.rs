use super::*;

impl<'a> LocalNegativeControlAuthority<'a> {
    pub(crate) fn bind(
        definitions: &'a ClaimDefinitions,
        context: &LiveContext,
        scratch_root: impl AsRef<Path>,
    ) -> Result<Self, String> {
        if definitions.registry_digest() != ADOPTED_CLAIM_REGISTRY_SHA256 {
            return Err("claims-control-registry-not-adopted".to_owned());
        }
        context
            .revalidate()
            .map_err(|_| "claims-control-live-context-invalid".to_owned())?;
        let context_id = context.context_id().to_owned();
        let candidate_id = candidate_id(context)?;
        if !digest(&context_id) || !digest(&candidate_id) {
            return Err("claims-control-candidate-binding-invalid".to_owned());
        }
        validate_private_control_root(scratch_root.as_ref())?;
        validate_registry_control_index(definitions)?;
        Ok(Self {
            definitions,
            context: context.clone(),
            context_id,
            candidate_id,
        })
    }

    pub fn context_id(&self) -> &str {
        &self.context_id
    }

    pub fn candidate_id(&self) -> &str {
        &self.candidate_id
    }

    pub fn execute_claim(
        &self,
        claim_id: &str,
    ) -> Result<BTreeMap<String, SemanticControlObservation>, String> {
        let _ = claim_id;
        #[cfg(not(test))]
        return product_executor_catalog_preflight();
        #[cfg(test)]
        Err(PRIVATE_TRANSPORT_UNAVAILABLE.to_owned())
    }

    /// Creates sealed semantic control models for decision-engine tests only.
    /// Models contain no process capture, exit status, executor identity,
    /// execution timestamp, output artifact, or causal outcome.
    #[cfg(test)]
    pub(crate) fn model_claim_for_decision_tests(
        &self,
        claim_id: &str,
    ) -> Result<BTreeMap<String, SemanticControlObservation>, String> {
        self.context
            .revalidate()
            .map_err(|_| "claims-control-live-context-invalid".to_owned())?;
        let definition = self
            .definitions
            .definition(claim_id)
            .ok_or_else(|| "claims-control-unknown-claim".to_owned())?;
        let mut observations = BTreeMap::new();
        for (ordinal, control_id) in definition.false_pass_controls.iter().enumerate() {
            let control = NamedControlDefinition::derive(
                self.definitions.registry_digest(),
                definition,
                control_id,
                ordinal,
            )?;
            let observation = self.model_control(&control)?;
            if observations
                .insert(control_id.clone(), observation)
                .is_some()
            {
                return Err("claims-control-catalog-duplicate".to_owned());
            }
        }
        self.context
            .revalidate()
            .map_err(|_| "claims-control-candidate-drift".to_owned())?;
        if self.context.context_id() != self.context_id
            || candidate_id(&self.context)? != self.candidate_id
        {
            return Err("claims-control-candidate-drift".to_owned());
        }
        Ok(observations)
    }

    #[cfg(test)]
    pub(crate) fn model_control(
        &self,
        control: &NamedControlDefinition,
    ) -> Result<SemanticControlObservation, String> {
        let sequence = MODEL_SEQUENCE.fetch_add(1, Ordering::SeqCst) + 1;
        let modeled_at_unix_ms = now_unix_ms()?;
        let nonce = digest_value(&format!(
            "claim-control-semantic-model-nonce-v1:{}:{}:{}:{}",
            control.definition_digest, self.context_id, self.candidate_id, sequence
        ));
        let plan =
            SemanticControlPlan::issue(control, &self.context_id, &self.candidate_id, &nonce)?;
        let authority = ModelAuthority::issue_for_decision_tests();
        let model_id = digest_value(&format!(
            "claim-control-semantic-model-id-v1:{}:{}:{}",
            plan.model_spec_digest, nonce, plan.modeled_result_digest
        ));
        let model = SemanticControlModel::from_authority(
            &authority,
            super::super::semantic_control_model_draft::SemanticControlModelDraft {
                model_id,
                registry_digest: self.definitions.registry_digest().to_owned(),
                claim_id: control.claim_id.clone(),
                control_id: control.control_id.clone(),
                control_definition_digest: control.definition_digest.clone(),
                model_spec_digest: plan.model_spec_digest,
                authority_nonce: nonce,
                negative_stimulus_digest: plan.negative_stimulus_digest,
                expected_failure_contract: control.expected_failure_contract.clone(),
                modeler: Actor {
                    actor_id: expected_modeler_id(control),
                    roles: BTreeSet::from([ActorRole::SemanticModeler]),
                },
                model_method: format!("{MODEL_METHOD}:{sequence}"),
                model_implementation_digest: plan.model_implementation_digest,
                modeled_at_unix_ms,
                live_context_id: self.context_id.clone(),
                candidate_id: self.candidate_id.clone(),
                max_age_ms: MODEL_MAX_AGE_MS,
                truth_surface: control.truth_surface.clone(),
                declared_ceiling: control.declared_ceiling.clone(),
                modeled_result_digest: plan.modeled_result_digest,
            },
        )?;
        SemanticControlObservation::from_authority(
            &authority,
            model,
            Actor {
                actor_id: model_observer_id(control),
                roles: BTreeSet::from([
                    ActorRole::IndependentObserver,
                    ActorRole::SemanticModelObserver,
                ]),
            },
            format!("{MODEL_OBSERVATION_METHOD}:{sequence}"),
            now_unix_ms()?.max(modeled_at_unix_ms),
        )
    }

    #[cfg(test)]
    pub(crate) fn execute_substituted_for_test(
        &self,
        claim_id: &str,
        control_id: &str,
        substitution: TestSubstitution,
    ) -> Result<SemanticControlObservation, String> {
        let _ = (claim_id, control_id, substitution);
        Err(PRIVATE_TRANSPORT_UNAVAILABLE.to_owned())
    }
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TestSubstitution {
    GenericTool,
    Arguments,
    FixtureSpec,
}

pub(crate) fn product_executor_catalog_preflight<T>() -> Result<T, String> {
    Err("claims-control-product-executor-catalog-unavailable".to_owned())
}

#[cfg(test)]
pub(crate) fn product_executor_catalog_preflight_for_test() -> Result<(), String> {
    product_executor_catalog_preflight()
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct NamedControlDefinition {
    pub(crate) registry_digest: String,
    pub(crate) claim_id: String,
    pub(crate) control_id: String,
    pub(crate) control_ordinal: usize,
    pub(crate) truth_surface: String,
    pub(crate) declared_ceiling: String,
    pub(crate) expected_failure_contract: String,
    pub(crate) definition_digest: String,
}
