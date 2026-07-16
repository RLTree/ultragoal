use super::*;

impl FileLedger {
    pub(crate) fn record_output_staged(
        &self,
        local: &mut LocalHead,
        token: &ReservationToken,
        relative_path: &str,
        identity: OutputDirectoryIdentity,
    ) -> Result<DurableWrite<()>, RoutineError> {
        self.record_output_transition(local, token, relative_path, identity, false)
    }

    pub(crate) fn record_output_component(
        &self,
        local: &mut LocalHead,
        token: &ReservationToken,
        relative_path: &str,
        identity: OutputDirectoryIdentity,
    ) -> Result<DurableWrite<()>, RoutineError> {
        self.record_output_transition(local, token, relative_path, identity, true)
    }

    fn record_output_transition(
        &self,
        local: &mut LocalHead,
        token: &ReservationToken,
        relative_path: &str,
        identity: OutputDirectoryIdentity,
        final_record: bool,
    ) -> Result<DurableWrite<()>, RoutineError> {
        if validate_output_journal(&token.output_journal).is_err() {
            return Err(error(
                "routine-production-output-journal-transition-invalid",
            ));
        }
        self.transition_payload(local, |payload, _tick, _head| {
            let record = exact_record_mut(payload, token)?;
            if !record.state.pending()
                || record.output_journal.root != token.output_journal.root
                || record.output_journal.scopes != token.output_journal.scopes
                || record.output_journal.components.len() != token.output_journal.components.len()
                || record
                    .output_journal
                    .components
                    .iter()
                    .zip(&token.output_journal.components)
                    .any(|(recorded, issued)| {
                        recorded.relative_path != issued.relative_path
                            || recorded.preexisting != issued.preexisting
                            || recorded.creation_nonce != issued.creation_nonce
                    })
            {
                return Err(error(
                    "routine-production-output-journal-transition-invalid",
                ));
            }
            let component = record
                .output_journal
                .components
                .iter_mut()
                .find(|component| component.relative_path == relative_path)
                .ok_or_else(|| error("routine-production-output-component-unbound"))?;
            if component.preexisting.is_some() {
                return Err(error("routine-production-output-component-preexisting"));
            }
            let destination = if final_record {
                if component.staged != Some(identity) {
                    return Err(error("routine-production-output-stage-identity-changed"));
                }
                &mut component.provisioned
            } else {
                &mut component.staged
            };
            match *destination {
                Some(observed) if observed != identity => {
                    return Err(error("routine-production-output-identity-changed"));
                }
                Some(_) => return Ok(((), false)),
                None => *destination = Some(identity),
            }
            Ok(((), true))
        })
    }
}
