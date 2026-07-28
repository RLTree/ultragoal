use super::*;

#[must_use = "the observed mediation must be settled by its production owner"]
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

#[must_use = "every prepared intent must receive one process observation"]
pub(crate) struct EffectMediation<'a> {
    context: &'a LiveContext,
    plan: &'a RoutinePlan,
    authority: RoutineMediationAuthority,
    intents: std::vec::IntoIter<RoutineMediatedIntent>,
    cancellation: RoutineCancellation,
    request_id: String,
    protocol_id: String,
    snapshot_id: String,
    dependencies: BTreeMap<String, String>,
    nodes: Vec<RoutineNodeMediation>,
    artifacts: Vec<Vec<u8>>,
    generated: Vec<(String, Vec<u8>)>,
    awaiting_observation: bool,
    incomplete: bool,
    cancelled: bool,
}

pub(crate) fn begin_effect_mediation<'a>(
    context: &'a LiveContext,
    plan: &'a RoutinePlan,
    request: RoutineEffectRequest,
    cancellation: RoutineCancellation,
) -> Result<EffectMediation<'a>, RoutineError> {
    preflight_request(context, plan, &request)?;
    let request_id = request.request_id().to_owned();
    let protocol_id = request.protocol_id().to_owned();
    let snapshot_id = request.snapshot_id.clone();
    let (authority, intents) = begin_routine_mediation(context, plan, request)?.into_parts();
    Ok(EffectMediation {
        context,
        plan,
        authority,
        intents: intents.into_iter(),
        cancellation,
        request_id,
        protocol_id,
        snapshot_id,
        dependencies: BTreeMap::new(),
        nodes: Vec::new(),
        artifacts: Vec::new(),
        generated: Vec::new(),
        awaiting_observation: false,
        incomplete: false,
        cancelled: false,
    })
}

impl EffectMediation<'_> {
    pub(crate) fn next_intent(&mut self) -> Result<Option<IntentExecutionRequest>, RoutineError> {
        if self.awaiting_observation {
            return Err(mediator_error("mediator-intent-observation-required"));
        }
        while let Some(token) = self.intents.next() {
            let expected_dependencies = token
                .intent()
                .expected_dependency_nodes()
                .iter()
                .map(|node| {
                    self.dependencies
                        .get(node)
                        .map(|digest| (node.clone(), digest.clone()))
                })
                .collect::<Option<BTreeMap<_, _>>>();
            if self.cancelled || self.cancellation.is_cancelled() {
                self.cancelled = true;
                self.record_incomplete(
                    token,
                    RoutineNodeDisposition::Cancelled,
                    "MEDIATOR-CANCELLED",
                )?;
                continue;
            }
            let Some(expected_dependencies) = expected_dependencies else {
                self.record_incomplete(
                    token,
                    RoutineNodeDisposition::DependencyFailed,
                    "MEDIATOR-DEPENDENCY-FAILED",
                )?;
                continue;
            };
            let prepared = match prepare_intent(
                self.context,
                self.plan,
                &token,
                &self.snapshot_id,
                expected_dependencies,
            ) {
                Ok(prepared) => prepared,
                Err(error) => {
                    let failure = error.cause().to_ascii_uppercase().replace('_', "-");
                    self.record_incomplete(token, RoutineNodeDisposition::Failed, failure)?;
                    continue;
                }
            };
            self.awaiting_observation = true;
            return Ok(Some(bind_intent_request(
                token,
                prepared,
                self.cancellation.clone(),
            )));
        }
        Ok(None)
    }

    pub(crate) fn observe_intent(
        &mut self,
        request: IntentExecutionRequest,
        observation: ProcessObservation,
    ) -> Result<(), RoutineError> {
        if !self.awaiting_observation {
            return Err(mediator_error("mediator-intent-observation-unexpected"));
        }
        let intent_id = request.intent_id().to_owned();
        let node_id = request.node_id().to_owned();
        let plan_order = request.plan_order();
        match request.observe(self.context, self.plan, observation)? {
            IntentResult::Executed(executed) => {
                self.dependencies
                    .insert(node_id.clone(), executed.result_sha256.clone());
                self.nodes.push(executed.node);
                let artifact_sha256 = sha256(&executed.reuse_bytes);
                self.generated
                    .push((artifact_sha256, executed.reuse_bytes.clone()));
                self.artifacts.push(executed.reuse_bytes);
            }
            IntentResult::Incomplete {
                disposition,
                failure_code,
            } => {
                self.incomplete = true;
                self.cancelled |= disposition == RoutineNodeDisposition::Cancelled;
                self.nodes.push(RoutineNodeMediation {
                    intent_id,
                    node_id,
                    plan_order,
                    disposition,
                    result_artifact_sha256: None,
                    failure_code: Some(failure_code.to_owned()),
                });
            }
        }
        self.awaiting_observation = false;
        Ok(())
    }

    pub(crate) fn finish(mut self) -> Result<ObservedRoutineMediation, RoutineError> {
        if self.awaiting_observation || self.intents.len() != 0 {
            return Err(mediator_error("mediator-intent-observation-incomplete"));
        }
        reconcile_internal(self.context, self.plan, &self.authority, &self.nodes)?;
        self.authority.finish()?;
        let settlement = if self.cancelled {
            DurableSettlement::Cancelled
        } else if self
            .nodes
            .iter()
            .any(|node| node.disposition == RoutineNodeDisposition::Failed)
        {
            DurableSettlement::Failed
        } else if self.incomplete {
            DurableSettlement::Incomplete
        } else {
            DurableSettlement::Complete
        };
        let authenticated = if settlement == DurableSettlement::Complete {
            collect_generated_witnesses(self.generated)
        } else {
            self.artifacts.clear();
            BTreeMap::new()
        };
        Ok(ObservedRoutineMediation {
            result: RoutineMediationResult {
                request_id: Some(self.request_id),
                protocol_id: Some(self.protocol_id),
                status: if self.cancelled {
                    RoutineMediatorStatus::Cancelled
                } else if self.incomplete {
                    RoutineMediatorStatus::IncompleteExecution
                } else {
                    RoutineMediatorStatus::CompleteExecution
                },
                nodes: self.nodes,
                recovery_marker: None,
                continuation: None,
                attempt_grant: None,
                checkpoint_head: None,
                terminal_outcome: None,
                support_limit: PRODUCTION_SUPPORT_LIMIT,
            },
            settlement,
            artifacts: self.artifacts,
            authenticated,
        })
    }

    fn record_incomplete(
        &mut self,
        token: RoutineMediatedIntent,
        disposition: RoutineNodeDisposition,
        failure_code: impl Into<String>,
    ) -> Result<(), RoutineError> {
        let node = incomplete_node(&token, disposition, failure_code);
        token.advance()?;
        self.incomplete = true;
        self.cancelled |= disposition == RoutineNodeDisposition::Cancelled;
        self.nodes.push(node);
        Ok(())
    }
}
