use super::*;

pub(crate) static CONTROL_MODELS: OnceLock<
    BTreeMap<String, BTreeMap<String, SemanticControlObservation>>,
> = OnceLock::new();

pub fn obligation_observation(
    definitions: &ClaimDefinitions,
    claim_id: &str,
    obligation: &ClaimObligation,
    ordinal: usize,
    tag: &str,
    false_pass_model: Option<SemanticControlObservation>,
) -> Observation {
    let definition = definitions.definition(claim_id).expect("definition");
    let evidence_id = format!("evidence-{claim_id}-{ordinal}-{tag}");
    assert_eq!(
        false_pass_model.is_some(),
        obligation.kind == ObligationKind::FalsePassControl,
        "false-pass semantic model must exactly match the obligation kind"
    );
    let result_digest = false_pass_model
        .as_ref()
        .map(|item| item.observed_digest().to_owned())
        .unwrap_or_else(|| digest(&format!("result:{evidence_id}")));
    let result = ObligationResult::Supported {
        result_digest: result_digest.clone(),
    };
    let (mut inputs, mut tools) = if let Some(observed) = &false_pass_model {
        let model = observed.model();
        (
            BTreeMap::from([(
                obligation.id.clone(),
                model.negative_stimulus_digest().to_owned(),
            )]),
            BTreeMap::from([(
                model.model_method().to_owned(),
                model.model_implementation_digest().to_owned(),
            )]),
        )
    } else {
        (
            BTreeMap::from([(
                "candidate-input".to_owned(),
                digest(&format!("input:{evidence_id}")),
            )]),
            BTreeMap::from([(
                "probe-tool".to_owned(),
                digest(&format!("tool:{evidence_id}")),
            )]),
        )
    };
    if matches!(
        obligation.kind,
        ObligationKind::RequiredSurface | ObligationKind::RequiredDecision
    ) {
        inputs.insert(
            obligation.id.clone(),
            digest(&format!("binding:{evidence_id}")),
        );
    }
    if obligation.kind == ObligationKind::RequiredTool {
        tools.insert(
            obligation.id.clone(),
            digest(&format!("tool-binding:{evidence_id}")),
        );
    }
    EvidenceEnvelope {
        evidence_id: evidence_id.clone(),
        claim_id: claim_id.to_owned(),
        obligation: obligation.clone(),
        producer: actor(
            &format!("producer-{claim_id}-{ordinal}-{tag}"),
            &[ActorRole::EvidenceProducer, ActorRole::IndependentObserver],
        ),
        method: format!("method-{claim_id}-{ordinal}-{tag}"),
        live_context_id: context_id().to_owned(),
        candidate_id: candidate_id().to_owned(),
        observed_at_unix_ms: now(),
        max_age_ms: 60_000,
        truth_surface: definition.truth_surface.clone(),
        declared_ceiling: definition.allowed_ceiling_on_pass.clone(),
        kind: EvidenceKind::DirectObservation,
        result,
        false_pass_model,
        inputs,
        environment_and_tools: tools,
        effects: BTreeMap::from([("read".to_owned(), "observed".to_owned())]),
        outputs: BTreeMap::from([(obligation.id.clone(), result_digest)]),
        artifact_digests: BTreeSet::from([digest(&format!("artifact:{evidence_id}"))]),
        limitations: vec![format!("bounded-to-{}", obligation.id)],
    }
    .observe()
    .expect("valid obligation observation")
}

pub fn observations_for(
    definitions: &ClaimDefinitions,
    claim_id: &str,
    tag: &str,
) -> Vec<Observation> {
    let mut models = if tag == "rerun" {
        authority(definitions)
            .model_claim_for_decision_tests(claim_id)
            .expect("rerun receives fresh named semantic control models")
    } else {
        cached_claim_controls(definitions, claim_id)
    };
    let observations = definitions
        .definition(claim_id)
        .expect("definition")
        .required_obligations()
        .iter()
        .enumerate()
        .map(|(index, obligation)| {
            let model = models.remove(&obligation.id);
            obligation_observation(definitions, claim_id, obligation, index, tag, model)
        })
        .collect();
    assert!(
        models.is_empty(),
        "all named semantic control models consumed exactly once"
    );
    observations
}

pub(crate) fn cached_claim_controls(
    definitions: &ClaimDefinitions,
    claim_id: &str,
) -> BTreeMap<String, SemanticControlObservation> {
    CONTROL_MODELS
        .get_or_init(|| {
            let bound = authority(definitions);
            definitions
                .order()
                .iter()
                .map(|claim_id| {
                    (
                        claim_id.clone(),
                        bound
                            .model_claim_for_decision_tests(claim_id)
                            .expect("authority models adopted named controls"),
                    )
                })
                .collect()
        })
        .get(claim_id)
        .cloned()
        .expect("cached adopted named controls")
}
