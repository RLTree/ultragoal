use super::super::terminal_settlement_fixture::{
    attempt_grant, grant_recovery_marker, recovering_grant,
};
use super::super::*;
use super::durable_authentication_fixture::*;
use std::sync::atomic::Ordering;

#[test]
fn durable_stage_failure_cannot_become_same_process_reuse_authority() {
    let durable = Arc::new(DurableRecord::default());
    durable.fail_stage.store(true, Ordering::SeqCst);
    let first = attempt_grant("durable-stage-failure", Some(durable.clone()), None);
    let protocol = first.protocol_id.clone();
    let (digest, witness, bytes) = generated_artifact("stage-failure");
    let marker = grant_recovery_marker(&first);
    let stage_error = run_reserved(&first, |attempt| {
        attempt.mark_started()?;
        let generated = collect_generated_witnesses(vec![(digest.clone(), bytes.clone())], attempt);
        attempt.stage_success(&generated)?;
        Ok((generated.clone(), ReservationTerminal::Complete(generated)))
    })
    .unwrap_err();
    assert_eq!(stage_error.cause(), "durable-authority-test-stage-failed");
    assert_eq!(
        observe_reservation(&protocol, None, Some(&digest)).non_durable_witness,
        None
    );
    assert!(durable.settlements.lock().unwrap().is_empty());
    assert_eq!(
        observe_reservation(&protocol, None, None).recovery_marker,
        Some(marker.clone())
    );

    let local = attempt_grant("non-durable-shadow", None, None);
    run_reserved(&local, |attempt| {
        drop(collect_generated_witnesses(
            vec![(digest.clone(), bytes.clone())],
            attempt,
        ));
        Ok((
            (),
            ReservationTerminal::Incomplete(DurableSettlement::Incomplete),
        ))
    })
    .unwrap();
    assert_eq!(
        observe_reservation(&protocol, None, Some(&digest)).non_durable_witness,
        Some(witness.clone())
    );

    let refused = recovering_grant(
        "durable-stage-refused",
        &protocol,
        marker.clone(),
        Some(durable.clone()),
    );
    let error = run_reserved(&refused, |attempt| {
        if attempt.authenticates_artifact(&digest, &witness)? {
            Ok((
                (),
                ReservationTerminal::Complete(BTreeMap::from([(digest.clone(), witness.clone())])),
            ))
        } else {
            Err(mediator_error("durable-artifact-witness-required"))
        }
    })
    .unwrap_err();
    assert_eq!(error.cause(), "durable-artifact-witness-required");
    assert!(durable.settlements.lock().unwrap().is_empty());

    durable.fail_stage.store(false, Ordering::SeqCst);
    let recovery = recovering_grant(
        "durable-stage-recovery",
        &protocol,
        marker,
        Some(durable.clone()),
    );
    let recovered = run_reserved(&recovery, |attempt| {
        attempt.mark_started()?;
        let generated = collect_generated_witnesses(vec![(digest.clone(), bytes)], attempt);
        attempt.stage_success(&generated)?;
        assert!(attempt.authenticates_artifact(&digest, &witness)?);
        assert!(!attempt.authenticates_artifact(&digest, &sha256(b"wrong-durable-witness"))?);
        Ok(((), ReservationTerminal::Complete(generated)))
    });
    assert!(recovered.is_ok(), "{recovered:?}");
    assert_eq!(
        durable.settlements.lock().unwrap().as_slice(),
        &[DurableSettlement::Complete]
    );
    assert_eq!(
        observe_reservation(&protocol, None, None).recovery_marker,
        None
    );
}

#[test]
fn process_authentication_is_confined_to_non_durable_mediation() {
    let grant = attempt_grant("non-durable-authentication", None, None);
    let (digest, witness, bytes) = generated_artifact("non-durable");
    run_reserved(&grant, |attempt| {
        collect_generated_witnesses(vec![(digest.clone(), bytes)], attempt);
        assert!(attempt.authenticates_artifact(&digest, &witness)?);
        Ok((
            (),
            ReservationTerminal::Incomplete(DurableSettlement::Incomplete),
        ))
    })
    .unwrap();
}
