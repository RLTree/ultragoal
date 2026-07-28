use super::*;

impl FileLedger {
    pub(in crate::routine_work::runtime_adapter::production::custody::store) fn settle_terminal(
        &self,
        local: &mut LocalHead,
        token: &ReservationToken,
        terminal: TerminalRecord,
    ) -> Result<DurableWrite<()>, RoutineError> {
        validate_terminal(&terminal)?;
        if terminal.failure_evidence.as_ref().is_some_and(|evidence| {
            evidence.protocol_id != token.binding.protocol_id
                || evidence.grant_id != token.grant_id
                || evidence.recovery_marker != token.recovery_marker
        }) {
            return Err(error("routine-production-terminal-failure-binding-invalid"));
        }
        let failure = terminal.failure_evidence.clone();
        self.transition_payload(
            local,
            PublicationContext::write(&token.grant_id, "terminal-settlement", failure.as_ref()),
            |payload, _tick, head| {
                let record = exact_record_mut(payload, token)?;
                if record.child.is_some()
                    || record.launch_stage.is_some()
                    || record.terminal.is_some()
                    || record.output_journal.components.iter().any(|component| {
                        component.staged.is_some() && component.provisioned != component.staged
                    })
                {
                    return Err(error("routine-production-live-custody-present"));
                }
                let valid_transition = match terminal.state {
                    AttemptState::Complete => {
                        record.state == AttemptState::Started
                            && record.next_intent == record.intents.len()
                            && !terminal.artifacts.is_empty()
                    }
                    AttemptState::Failed | AttemptState::Cancelled | AttemptState::Incomplete => {
                        terminal.artifacts.is_empty()
                            && matches!(
                                record.state,
                                AttemptState::Reserved
                                    | AttemptState::Staged
                                    | AttemptState::Started
                            )
                    }
                    AttemptState::RolledBack => {
                        terminal.artifacts.is_empty() && record.state == AttemptState::Reserved
                    }
                    AttemptState::Reserved
                    | AttemptState::Staged
                    | AttemptState::Started
                    | AttemptState::Ambiguous => false,
                };
                if !valid_transition || terminal.prior_head_sha256 != head {
                    return Err(error("routine-production-settlement-transition-invalid"));
                }
                record.state = terminal.state;
                record.terminal = Some(terminal);
                Ok(((), true))
            },
        )
    }
}

fn validate_terminal(terminal: &TerminalRecord) -> Result<(), RoutineError> {
    if terminal.state.pending()
        || terminal.state == AttemptState::Ambiguous
        || !valid(&terminal.result_sha256)
        || !valid(&terminal.prior_head_sha256)
        || terminal
            .artifacts
            .iter()
            .any(|(digest, witness)| !valid(digest) || !valid(witness))
        || !terminal.process_cleanup.shape_is_valid()
        || !terminal.staged_cleanup.shape_is_valid()
        || terminal.failure_evidence.as_ref().is_some_and(|evidence| {
            !evidence.shape_is_valid()
                || terminal.state != AttemptState::Failed
                || terminal.process_cleanup != evidence.process_cleanup
                || terminal.staged_cleanup != evidence.staged_cleanup
        })
        || (terminal.state == AttemptState::Complete) != terminal.mediation.is_some()
        || terminal.mediation.as_ref().is_some_and(|mediation| {
            mediation.nodes.is_empty()
                || mediation.nodes.iter().any(|node| {
                    node.intent_id.is_empty()
                        || node.node_id.is_empty()
                        || !valid(&node.result_artifact_sha256)
                })
        })
    {
        return Err(error("routine-production-settlement-invalid"));
    }
    Ok(())
}
