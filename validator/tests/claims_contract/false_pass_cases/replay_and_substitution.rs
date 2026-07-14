use super::*;

#[test]
pub(crate) fn model_replay_is_rejected_for_the_sealed_semantic_model() {
    let definitions = definitions();
    let claim_id = "CL-PACKAGE";
    let mut ledger = super::super::scenario::semantic_model_ledger();
    pass_before(&mut ledger, &definitions, claim_id, "model-replay-prior");
    let mut observations = observations_for(&definitions, claim_id, "model-replay");
    let position = false_pass_position(&observations);
    let mut duplicate = observations[position].envelope().clone();
    duplicate.evidence_id.push_str("-duplicate");
    duplicate.method.push_str("-duplicate");
    duplicate.producer.actor_id.push_str("-duplicate");
    duplicate.artifact_digests = [digest("duplicate-envelope-artifact")]
        .into_iter()
        .collect();
    observations.push(duplicate.observe().expect("duplicate envelope"));
    let decision = decide(&mut ledger, &definitions, claim_id, observations);
    assert_eq!(decision.status, DecisionStatus::Rejected);
    assert!(
        decision
            .reasons
            .iter()
            .any(|reason| reason.contains("model-replayed"))
    );
}

#[test]
pub(crate) fn sealed_semantic_model_indexes_every_adopted_claims_exact_control_set() {
    let definitions = definitions();
    let authority = authority(&definitions);
    for claim_id in definitions.order() {
        let expected = definitions
            .definition(claim_id)
            .expect("adopted definition")
            .false_pass_controls
            .iter()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>();
        let observed = authority
            .model_claim_for_decision_tests(claim_id)
            .expect("sealed semantic controls model");
        assert_eq!(
            observed
                .keys()
                .cloned()
                .collect::<std::collections::BTreeSet<_>>(),
            expected
        );
        for model in observed.values().map(SemanticControlObservation::model) {
            assert_eq!(model.registry_digest(), definitions.registry_digest());
            assert_eq!(model.claim_id(), claim_id);
            assert_eq!(
                model.truth_surface(),
                definitions.definition(claim_id).unwrap().truth_surface
            );
            assert_eq!(
                model.declared_ceiling(),
                definitions
                    .definition(claim_id)
                    .unwrap()
                    .allowed_ceiling_on_pass
            );
        }
    }
}

#[test]
pub(crate) fn generic_tool_argument_and_fixture_spec_substitution_fail_before_model_or_proof_issue()
{
    let definitions = definitions();
    let authority = authority(&definitions);
    for claim_id in definitions.order() {
        let control_id = definitions
            .definition(claim_id)
            .unwrap()
            .false_pass_controls
            .first()
            .unwrap();
        assert_eq!(
            authority
                .execute_substituted_for_test(claim_id, control_id, TestSubstitution::GenericTool,)
                .unwrap_err(),
            PRIVATE_TRANSPORT_UNAVAILABLE,
            "generic failure substitution:{claim_id}"
        );
    }
    let claim_id = "CL-SOURCE";
    let control_id = definitions
        .definition(claim_id)
        .unwrap()
        .false_pass_controls
        .first()
        .unwrap();
    for substitution in [TestSubstitution::Arguments, TestSubstitution::FixtureSpec] {
        assert_eq!(
            authority
                .execute_substituted_for_test(claim_id, control_id, substitution)
                .unwrap_err(),
            PRIVATE_TRANSPORT_UNAVAILABLE,
            "{substitution:?}"
        );
    }
}

#[test]
pub(crate) fn production_preflight_rejects_test_transport_as_product_behavior() {
    assert_eq!(
        product_executor_catalog_preflight_for_test(),
        Err("claims-control-product-executor-catalog-unavailable".to_owned())
    );
}

#[test]
pub(crate) fn production_claims_module_compiles_without_dormant_process_transport() {
    let source = include_str!("../../../src/claims/false_pass/mod.rs");
    for prohibited in [
        "FixtureScheduler",
        "FixtureExecutor",
        "ConfinementPlan",
        "ProcessCapture",
        "RunDisposition",
        "execute_control",
        "run_substituted_fixture",
        "allow(unreachable_code)",
    ] {
        assert!(
            !source.contains(prohibited),
            "dormant transport:{prohibited}"
        );
    }
    let library = include_str!("../../../src/lib.rs");
    assert!(library.lines().any(|line| line.trim() == "mod claims;"));
}

#[test]
pub(crate) fn private_transport_refuses_before_schedule_model_or_proof_issue() {
    let definitions = definitions();
    let authority = authority(&definitions);
    let before = scratch_snapshot(control_scratch_root());

    assert_eq!(
        authority.execute_claim("CL-SOURCE").unwrap_err(),
        PRIVATE_TRANSPORT_UNAVAILABLE
    );

    let after = scratch_snapshot(control_scratch_root());
    assert_eq!(after, before);
}

#[test]
pub(crate) fn semantic_model_uses_current_digest_candidate_bindings_and_unique_model_ids() {
    let definitions = std::sync::Arc::new(definitions());
    let bound_authority = authority(&definitions);
    assert!(bound_authority.context_id().starts_with("sha256:"));
    assert!(bound_authority.candidate_id().starts_with("sha256:"));
    assert_ne!(bound_authority.context_id(), "arbitrary-context");
    assert_ne!(bound_authority.candidate_id(), "arbitrary-candidate");
    drop(bound_authority);

    let handles = (0..4)
        .map(|_| {
            let definitions = std::sync::Arc::clone(&definitions);
            std::thread::spawn(move || {
                authority(&definitions)
                    .model_claim_for_decision_tests("CL-SOURCE")
                    .expect("concurrent sealed semantic model")
                    .into_values()
                    .map(|observed| observed.model().model_id().to_owned())
                    .collect::<Vec<_>>()
            })
        })
        .collect::<Vec<_>>();
    let mut ids = std::collections::BTreeSet::new();
    for handle in handles {
        for id in handle.join().expect("authority thread") {
            assert!(ids.insert(id), "model ids are unique under concurrency");
        }
    }
    assert_eq!(ids.len(), 20);
}

#[test]
pub(crate) fn cross_claim_named_control_model_rejects() {
    let definitions = definitions();
    let mut source = observations_for(&definitions, "CL-SOURCE", "cross-claim-source");
    let source_position = false_pass_position(&source);
    let foreign = source
        .remove(source_position)
        .envelope()
        .false_pass_model
        .as_ref()
        .expect("source control")
        .clone();
    let claim_id = "CL-PACKAGE";
    let mut ledger = super::super::scenario::semantic_model_ledger();
    pass_before(&mut ledger, &definitions, claim_id, "cross-claim-prior");
    let mut package = observations_for(&definitions, claim_id, "cross-claim");
    let position = false_pass_position(&package);
    let mut envelope = package.remove(position).envelope().clone();
    envelope.false_pass_model = Some(foreign);
    envelope.result = ObligationResult::Supported {
        result_digest: envelope
            .false_pass_model
            .as_ref()
            .unwrap()
            .observed_digest()
            .to_owned(),
    };
    envelope.outputs = BTreeMap::from([(
        envelope.obligation.id.clone(),
        envelope.result.result_digest().to_owned(),
    )]);
    package.push(envelope.observe().expect("cross-claim envelope shape"));
    assert_eq!(
        decide(&mut ledger, &definitions, claim_id, package).status,
        DecisionStatus::Rejected
    );
}
