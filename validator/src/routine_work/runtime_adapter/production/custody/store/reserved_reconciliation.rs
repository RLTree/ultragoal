use super::*;

impl FileLedger {
    pub(in crate::routine_work::runtime_adapter::production::custody::store) fn reconcile_reserved(
        &self,
        local: &mut LocalHead,
        binding: &AuthorityBinding,
        attempt_grant: &str,
        expected_head: &str,
    ) -> Result<DurableWrite<ContinuationDisposition>, RoutineError> {
        validate_binding(binding)?;
        if !valid(attempt_grant) || !valid(expected_head) {
            return Err(error("routine-production-continuation-checkpoint-invalid"));
        }
        self.transition_payload(local, PublicationContext::read(), |payload, _tick, head| {
            if head != expected_head {
                return Err(error("routine-production-continuation-ledger-stale"));
            }
            let record = payload
                .attempts
                .get_mut(attempt_grant)
                .ok_or_else(|| error("routine-production-continuation-record-invalid"))?;
            if record.binding != *binding {
                return Err(error("routine-production-continuation-binding-invalid"));
            }
            if record.state == AttemptState::Complete
                && record.terminal.as_ref().is_some_and(|terminal| {
                    terminal.state == AttemptState::Complete
                        && !terminal.artifacts.is_empty()
                        && terminal.mediation.is_some()
                        && terminal.failure_evidence.is_none()
                })
            {
                return Ok((
                    ContinuationDisposition::Complete(reuse_result(record, head)?),
                    false,
                ));
            }
            if record.state != AttemptState::Reserved
                || record.child.is_some()
                || record.launch_stage.is_some()
                || record
                    .output_journal
                    .components
                    .iter()
                    .any(|component| component.staged.is_some() || component.provisioned.is_some())
            {
                return Err(error("routine-production-continuation-effect-ambiguous"));
            }
            if record.terminal.is_some() || record.publication_ambiguity.is_some() {
                return Err(error("routine-production-continuation-record-invalid"));
            }
            record.state = AttemptState::RolledBack;
            Ok((ContinuationDisposition::Reserved, true))
        })
    }
}

#[derive(Debug)]
pub(in crate::routine_work::runtime_adapter::production::custody) enum ContinuationDisposition {
    Complete(crate::routine_work::runtime_adapter::mediator::RoutineMediationResult),
    Reserved,
}

fn reuse_result(
    record: &ProtocolRecord,
    authenticated_head: &str,
) -> Result<crate::routine_work::runtime_adapter::mediator::RoutineMediationResult, RoutineError> {
    let terminal = record
        .terminal
        .as_ref()
        .ok_or_else(|| error("routine-production-continuation-record-invalid"))?;
    let mediation = terminal
        .mediation
        .as_ref()
        .ok_or_else(|| error("routine-production-continuation-record-invalid"))?;
    let nodes = mediation
        .nodes
        .iter()
        .map(
            |node| crate::routine_work::runtime_adapter::mediator::RoutineNodeMediation {
                intent_id: node.intent_id.clone(),
                node_id: node.node_id.clone(),
                plan_order: node.plan_order,
                disposition:
                    crate::routine_work::runtime_adapter::mediator::RoutineNodeDisposition::Reused,
                result_artifact_sha256: Some(node.result_artifact_sha256.clone()),
                failure_code: None,
            },
        )
        .collect();
    let continuation = crate::routine_work::digest::digest_of(&(
        "routine-public-continuation-v1",
        &record.binding.protocol_id,
        &record.grant_id,
        &record.recovery_marker,
    ))?;
    Ok(crate::routine_work::runtime_adapter::mediator::RoutineMediationResult {
        request_id: Some(record.request_id.clone()),
        protocol_id: Some(record.binding.protocol_id.clone()),
        status: crate::routine_work::runtime_adapter::mediator::RoutineMediatorStatus::CompleteExecution,
        nodes,
        recovery_marker: None,
        continuation: Some(format!("routine-cont-{continuation}")),
        attempt_grant: Some(record.grant_id.clone()),
        checkpoint_head: Some(authenticated_head.to_owned()),
        support_limit: crate::routine_work::runtime_adapter::mediator::PRODUCTION_SUPPORT_LIMIT,
    })
}
