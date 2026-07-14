use super::*;

pub(crate) fn mediation_tokens(
    request_id: &str,
    protocol_id: &str,
    execution_result_scope: &str,
    intents: Vec<RoutineEffectIntent>,
    seal: &Arc<RequestSeal>,
) -> Vec<RoutineMediatedIntent> {
    intents
        .into_iter()
        .map(|intent| {
            RoutineMediatedIntent::new(
                request_id.to_owned(),
                protocol_id.to_owned(),
                execution_result_scope.to_owned(),
                intent,
                Arc::clone(seal),
            )
        })
        .collect()
}

pub(crate) fn authority_seal(request: &RoutineEffectRequest) -> Arc<RequestSeal> {
    Arc::clone(&request.seal)
}

pub(crate) fn request_error(cause: &'static str) -> RoutineError {
    RoutineError::new(RoutineErrorId::InvalidRequest, cause, None)
}

pub(crate) fn duplicate_node(values: &[RoutineInvocationSpec]) -> Option<&str> {
    let mut seen = BTreeSet::new();
    values
        .iter()
        .find_map(|value| (!seen.insert(value.node_id.as_str())).then_some(value.node_id.as_str()))
}
