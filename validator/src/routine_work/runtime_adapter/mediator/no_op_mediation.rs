use super::*;

pub(crate) fn mediate_noop(
    context: &LiveContext,
    plan: &RoutinePlan,
    projection: RoutineNoOpProjection,
    grant: Option<RoutineRootGrant>,
    reuse: RoutineReuseInput,
) -> Result<RoutineMediationResult, RoutineError> {
    if grant.is_some() || !reuse.into_artifacts().is_empty() {
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

pub(crate) fn mediate_effect(
    context: &LiveContext,
    plan: &RoutinePlan,
    request: RoutineEffectRequest,
    grant: Option<RoutineRootGrant>,
    cancellation: RoutineCancellation,
    reuse: RoutineReuseInput,
    publisher: Option<&dyn RoutineArtifactPublisher>,
) -> Result<RoutineMediationResult, RoutineError> {
    let grant = grant.ok_or_else(|| mediator_error("mediator-root-grant-missing"))?;
    let production = grant.durable.is_some();
    validate_grant(context, plan, &request, &grant)?;
    let supplied_reuse = index_reuse_inputs(reuse, &request)?;
    preflight_request(context, plan, &request)?;
    run_test_pre_spawn_hook();
    preflight_request(context, plan, &request)?;
    let request_id = request.request_id().to_owned();
    let protocol_id = request.protocol_id().to_owned();
    let attempt = reserve_grant(&grant)?;
    run_reserved(attempt, |attempt| {
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
            if cancelled || cancellation.is_cancelled() {
                cancelled = true;
                incomplete = true;
                advance_and_cleanup(&token, &attempt)?;
                nodes.push(incomplete_node(
                    &token,
                    RoutineNodeDisposition::Cancelled,
                    "MEDIATOR-CANCELLED",
                ));
                continue;
            }
            let Some(expected_dependencies) = expected_dependencies else {
                incomplete = true;
                advance_and_cleanup(&token, &attempt)?;
                nodes.push(incomplete_node(
                    &token,
                    RoutineNodeDisposition::DependencyFailed,
                    "MEDIATOR-DEPENDENCY-FAILED",
                ));
                continue;
            };
            let result = mediate_intent(
                context,
                plan,
                &token,
                &snapshot_id,
                &expected_dependencies,
                supplied_reuse.get(token.intent().intent_id()),
                &cancellation,
                &attempt,
            );
            match result {
                Ok(IntentResult::Reused(verified)) => {
                    let result_sha256 = verified.wire.result_artifact_sha256.clone();
                    dependencies.insert(token.intent().node_id().to_owned(), result_sha256.clone());
                    nodes.push(success_node(
                        &token,
                        RoutineNodeDisposition::Reused,
                        result_sha256,
                    ));
                    let artifact_sha256 = sha256(&verified.canonical_bytes);
                    generated.push((artifact_sha256, verified.canonical_bytes.clone()));
                    artifacts.push(verified.canonical_bytes);
                    advance_and_cleanup(&token, &attempt)?;
                }
                Ok(IntentResult::Executed(executed)) => {
                    dependencies.insert(
                        token.intent().node_id().to_owned(),
                        executed.result_sha256.clone(),
                    );
                    nodes.push(success_node(
                        &token,
                        RoutineNodeDisposition::Executed,
                        executed.result_sha256,
                    ));
                    let artifact_sha256 = sha256(&executed.reuse_bytes);
                    generated.push((artifact_sha256, executed.reuse_bytes.clone()));
                    artifacts.push(executed.reuse_bytes);
                    advance_and_cleanup(&token, &attempt)?;
                }
                Ok(IntentResult::Incomplete {
                    disposition,
                    failure_code,
                    started,
                }) => {
                    incomplete = true;
                    cancelled |= disposition == RoutineNodeDisposition::Cancelled;
                    debug_assert!(!started || attempt.started.get());
                    advance_and_cleanup(&token, &attempt)?;
                    nodes.push(incomplete_node(&token, disposition, failure_code));
                }
                Err(error) => {
                    incomplete = true;
                    advance_and_cleanup(&token, &attempt)?;
                    nodes.push(incomplete_node(
                        &token,
                        RoutineNodeDisposition::Failed,
                        error.cause().to_ascii_uppercase().replace('_', "-"),
                    ));
                }
            }
        }
        reconcile_internal(context, plan, &authority, &nodes)?;
        run_test_finish_failure_hook(&authority);
        authority.finish()?;
        // The process has been reaped and the batch outcome is reconciled. Consume
        // every launch snapshot before any terminal ledger transition; a custody
        // failure therefore leaves the exact Started reservation recoverable.
        attempt.cleanup_staged()?;
        let recovery_marker = if incomplete {
            artifacts.clear();
            generated.clear();
            attempt.settle_incomplete(if cancelled {
                DurableSettlement::Cancelled
            } else if nodes
                .iter()
                .any(|node| node.disposition == RoutineNodeDisposition::Failed)
            {
                DurableSettlement::Failed
            } else {
                DurableSettlement::Incomplete
            })?
        } else {
            let authenticated = collect_generated_witnesses(generated, attempt);
            if !attempt.reuse_only() {
                attempt.stage_success(&authenticated)?;
                if let Some(publisher) = publisher {
                    publisher.publish(&artifacts)?;
                }
            }
            let settlement_artifacts = if attempt.reuse_only() {
                BTreeMap::new()
            } else {
                authenticated
            };
            attempt.settle_success(&settlement_artifacts)?;
            None
        };
        Ok(RoutineMediationResult {
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
            recovery_marker,
            support_limit: if production {
                PRODUCTION_SUPPORT_LIMIT
            } else {
                MEDIATOR_SUPPORT_LIMIT
            },
        })
    })
}

fn advance_and_cleanup(
    token: &RoutineMediatedIntent,
    attempt: &AttemptReservation,
) -> Result<(), RoutineError> {
    let advanced = token.advance();
    let cleanup = attempt.cleanup_staged();
    match (advanced, cleanup) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), Ok(())) => Err(error),
        (Ok(()), Err(error)) => Err(error),
        (Err(_), Err(error)) => Err(error),
    }
}

pub(crate) enum IntentResult {
    Reused(VerifiedReuseArtifact),
    Executed(ExecutedArtifact),
    Incomplete {
        disposition: RoutineNodeDisposition,
        failure_code: &'static str,
        started: bool,
    },
}
