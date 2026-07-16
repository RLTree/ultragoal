use super::*;

pub(crate) fn mediate_noop(
    context: &LiveContext,
    plan: &RoutinePlan,
    projection: RoutineNoOpProjection,
    reuse: RoutineReuseInput,
) -> Result<RoutineMediationResult, RoutineError> {
    if !reuse.into_artifacts().is_empty() {
        return Err(mediator_error("mediator-noop-authority-or-reuse-present"));
    }
    let current = RoutineBinding::from_live(context)?;
    if &current != plan.binding()
        || projection.context_id() != context.context_id()
        || projection.candidate_id() != plan.binding().candidate_id()
        || projection.plan_id() != plan.plan_id()
        || !projection.selected().is_empty()
        || projection.effect_intent_count() != 0
    {
        return Err(RoutineError::new(
            RoutineErrorId::ContextMismatch,
            "mediator-noop-binding-invalid",
            None,
        ));
    }
    Ok(RoutineMediationResult {
        request_id: None,
        protocol_id: None,
        status: RoutineMediatorStatus::CompleteNoOp,
        nodes: Vec::new(),
        recovery_marker: None,
        support_limit: MEDIATOR_SUPPORT_LIMIT,
    })
}

pub(crate) struct ObservedRoutineMediation {
    result: RoutineMediationResult,
    settlement: DurableSettlement,
    artifacts: Vec<Vec<u8>>,
    authenticated: BTreeMap<String, String>,
}

impl ObservedRoutineMediation {
    pub(crate) fn into_parts(
        self,
    ) -> (
        RoutineMediationResult,
        DurableSettlement,
        Vec<Vec<u8>>,
        BTreeMap<String, String>,
    ) {
        (
            self.result,
            self.settlement,
            self.artifacts,
            self.authenticated,
        )
    }
}

pub(crate) fn mediate_effect_observed(
    context: &LiveContext,
    plan: &RoutinePlan,
    request: RoutineEffectRequest,
    cancellation: RoutineCancellation,
    reuse: RoutineReuseInput,
    attempt: &RoutineExecutionCapability<'_>,
) -> Result<ObservedRoutineMediation, RoutineError> {
    let supplied_reuse = index_reuse_inputs(reuse, &request)?;
    preflight_request(context, plan, &request)?;
    let request_id = request.request_id().to_owned();
    let protocol_id = request.protocol_id().to_owned();
    let snapshot_id = request.snapshot_id.clone();
    let batch = begin_routine_mediation(context, plan, request)?;
    let (authority, intents) = batch.into_parts();
    let mut dependencies = BTreeMap::<String, String>::new();
    let mut nodes = Vec::with_capacity(intents.len());
    let mut artifacts = Vec::<Vec<u8>>::with_capacity(intents.len());
    let mut generated = Vec::<(String, Vec<u8>)>::new();
    let mut incomplete = false;
    let mut cancelled = false;

    for token in intents {
        let expected_dependencies = token
            .intent()
            .expected_dependency_nodes()
            .iter()
            .map(|node| {
                dependencies
                    .get(node)
                    .map(|digest| (node.clone(), digest.clone()))
            })
            .collect::<Option<BTreeMap<_, _>>>();
        let node = if cancelled || cancellation.is_cancelled() {
            cancelled = true;
            incomplete = true;
            incomplete_node(
                &token,
                RoutineNodeDisposition::Cancelled,
                "MEDIATOR-CANCELLED",
            )
        } else if let Some(expected_dependencies) = expected_dependencies {
            let result = mediate_intent(
                context,
                plan,
                &token,
                &snapshot_id,
                &expected_dependencies,
                supplied_reuse.get(token.intent().intent_id()),
                &cancellation,
                attempt,
            );
            match result {
                Ok(IntentResult::Reused(verified)) => {
                    let result_sha256 = verified.wire.result_artifact_sha256.clone();
                    dependencies.insert(token.intent().node_id().to_owned(), result_sha256.clone());
                    let node = success_node(&token, RoutineNodeDisposition::Reused, result_sha256);
                    let artifact_sha256 = sha256(&verified.canonical_bytes);
                    generated.push((artifact_sha256, verified.canonical_bytes.clone()));
                    artifacts.push(verified.canonical_bytes);
                    node
                }
                Ok(IntentResult::Executed(executed)) => {
                    dependencies.insert(
                        token.intent().node_id().to_owned(),
                        executed.result_sha256.clone(),
                    );
                    let node = success_node(
                        &token,
                        RoutineNodeDisposition::Executed,
                        executed.result_sha256,
                    );
                    let artifact_sha256 = sha256(&executed.reuse_bytes);
                    generated.push((artifact_sha256, executed.reuse_bytes.clone()));
                    artifacts.push(executed.reuse_bytes);
                    node
                }
                Ok(IntentResult::Incomplete {
                    disposition,
                    failure_code,
                }) => {
                    incomplete = true;
                    cancelled |= disposition == RoutineNodeDisposition::Cancelled;
                    incomplete_node(&token, disposition, failure_code)
                }
                Err(error) => {
                    incomplete = true;
                    incomplete_node(
                        &token,
                        RoutineNodeDisposition::Failed,
                        error.cause().to_ascii_uppercase().replace('_', "-"),
                    )
                }
            }
        } else {
            incomplete = true;
            incomplete_node(
                &token,
                RoutineNodeDisposition::DependencyFailed,
                "MEDIATOR-DEPENDENCY-FAILED",
            )
        };
        nodes.push(complete_intent_transition(&token, attempt, node)?);
    }
    reconcile_internal(context, plan, &authority, &nodes)?;
    authority.finish()?;
    attempt.observe_staged_transition(|| Ok(()))?;
    let settlement = if cancelled {
        DurableSettlement::Cancelled
    } else if nodes
        .iter()
        .any(|node| node.disposition == RoutineNodeDisposition::Failed)
    {
        DurableSettlement::Failed
    } else if incomplete {
        DurableSettlement::Incomplete
    } else {
        DurableSettlement::Complete
    };
    let authenticated = if settlement == DurableSettlement::Complete {
        collect_generated_witnesses(generated)
    } else {
        artifacts.clear();
        BTreeMap::new()
    };
    Ok(ObservedRoutineMediation {
        result: RoutineMediationResult {
            request_id: Some(request_id),
            protocol_id: Some(protocol_id),
            status: if cancelled {
                RoutineMediatorStatus::Cancelled
            } else if incomplete {
                RoutineMediatorStatus::IncompleteExecution
            } else {
                RoutineMediatorStatus::CompleteExecution
            },
            nodes,
            recovery_marker: None,
            support_limit: PRODUCTION_SUPPORT_LIMIT,
        },
        settlement,
        artifacts,
        authenticated,
    })
}

fn complete_intent_transition(
    token: &RoutineMediatedIntent,
    attempt: &RoutineExecutionCapability<'_>,
    node: RoutineNodeMediation,
) -> Result<RoutineNodeMediation, RoutineError> {
    attempt.observe_staged_transition(|| token.advance())?;
    Ok(node)
}

pub(crate) enum IntentResult {
    Reused(VerifiedReuseArtifact),
    Executed(ExecutedArtifact),
    Incomplete {
        disposition: RoutineNodeDisposition,
        failure_code: &'static str,
    },
}
