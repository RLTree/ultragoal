use super::super::terminal_settlement_fixture::attempt;
use super::super::*;
use super::durable_authentication_fixture::*;
use std::sync::atomic::Ordering;

#[test]
fn durable_stage_failure_cannot_become_same_process_reuse_authority() {
    let durable = Arc::new(DurableRecord::default());
    durable.fail_stage.store(true, Ordering::SeqCst);
    let first = attempt("durable-stage-failure", Some(durable.clone()), false, None);
    let protocol = first.protocol_id().clone();
    let marker = first.recovery_marker().clone();
    let (digest, witness, bytes) = generated_artifact("stage-failure");
    let stage_error = run_reserved(first, |attempt| {
        attempt.mark_started()?;
        let generated = collect_generated_witnesses(vec![(digest.clone(), bytes.clone())], attempt);
        attempt.stage_success(&generated)?;
        attempt.settle_success(&generated)?;
        Ok(generated)
    })
    .unwrap_err();
    assert_eq!(stage_error.cause(), "durable-authority-test-stage-failed");
    assert!(
        !registry()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .non_durable_authenticated_artifacts
            .contains_key(&digest)
    );
    assert!(
        durable
            .settlements
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .is_empty()
    );
    {
        let state = registry()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        assert!(!state.active_protocols.contains_key(&protocol));
        assert_eq!(state.ambiguous_protocols.get(&protocol), Some(&marker));
    }

    registry()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .non_durable_authenticated_artifacts
        .insert(digest.clone(), witness.clone());
    let refused = attempt(
        "durable-stage-failure",
        Some(durable.clone()),
        false,
        Some(marker.clone()),
    );
    let error = run_reserved(refused, |attempt| {
        if attempt.authenticates_artifact(&digest, &witness)? {
            attempt.settle_success(&BTreeMap::from([(digest.clone(), witness.clone())]))?;
            Ok(())
        } else {
            Err(mediator_error("durable-artifact-witness-required"))
        }
    })
    .unwrap_err();
    assert_eq!(error.cause(), "durable-artifact-witness-required");
    assert!(
        durable
            .settlements
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .is_empty()
    );

    durable.fail_stage.store(false, Ordering::SeqCst);
    registry()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .non_durable_authenticated_artifacts
        .insert(digest.clone(), sha256(b"conflicting-process-witness"));
    let recovery = attempt(
        "durable-stage-failure",
        Some(durable.clone()),
        false,
        Some(marker),
    );
    let recovered = run_reserved(recovery, |attempt| {
        attempt.mark_started()?;
        let generated = collect_generated_witnesses(vec![(digest.clone(), bytes)], attempt);
        attempt.stage_success(&generated)?;
        assert!(attempt.authenticates_artifact(&digest, &witness)?);
        assert!(!attempt.authenticates_artifact(&digest, &sha256(b"wrong-durable-witness"))?);
        attempt.settle_success(&generated)?;
        Ok(())
    });
    assert!(recovered.is_ok(), "{recovered:?}");
    assert_eq!(
        durable
            .settlements
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_slice(),
        &[DurableSettlement::Complete]
    );
    assert!(
        !registry()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .ambiguous_protocols
            .contains_key(&protocol)
    );
    registry()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .non_durable_authenticated_artifacts
        .remove(&digest);
}

#[test]
fn process_authentication_is_confined_to_non_durable_mediation() {
    let attempt = attempt("non-durable-authentication", None, false, None);
    let (digest, witness, bytes) = generated_artifact("non-durable");
    collect_generated_witnesses(vec![(digest.clone(), bytes)], &attempt);
    assert!(attempt.authenticates_artifact(&digest, &witness).unwrap());
    registry()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .non_durable_authenticated_artifacts
        .remove(&digest);
}
