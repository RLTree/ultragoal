use super::*;

pub(crate) fn incomplete_node(
    token: &RoutineMediatedIntent,
    disposition: RoutineNodeDisposition,
    failure_code: impl Into<String>,
) -> RoutineNodeMediation {
    RoutineNodeMediation {
        intent_id: token.intent().intent_id().to_owned(),
        node_id: token.intent().node_id().to_owned(),
        plan_order: token.intent().plan_order(),
        disposition,
        result_artifact_sha256: None,
        failure_code: Some(failure_code.into()),
    }
}

pub(super) fn collect_generated_witnesses(
    values: Vec<(String, Vec<u8>)>,
    attempt: &ReservationAttempt<'_>,
) -> BTreeMap<String, String> {
    let mut authenticated = BTreeMap::new();
    for (digest, bytes) in values {
        if let Ok(wire) = serde_json::from_slice::<ReuseArtifactWire>(&bytes) {
            authenticated.insert(digest, wire.mediator_witness_sha256);
        }
    }
    attempt.retain_non_durable_authentication(&authenticated);
    authenticated
}

pub(crate) fn recovery_identity(grant_id: &str, protocol_id: &str, request_id: &str) -> String {
    framed(&[
        RECOVERY_DOMAIN,
        grant_id.as_bytes(),
        protocol_id.as_bytes(),
        request_id.as_bytes(),
    ])
}

pub(crate) fn mediator_error(cause: &'static str) -> RoutineError {
    RoutineError::new(RoutineErrorId::InvalidRequest, cause, None)
}

pub(crate) fn concurrent(cause: &'static str) -> RoutineError {
    RoutineError::new(RoutineErrorId::ConcurrentMutation, cause, None)
}

#[cfg(test)]
pub(crate) use filesystem::{set_test_output_capture_hook, set_test_read_source_capture_hook};
