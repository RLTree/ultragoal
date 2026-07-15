use super::*;

impl FileLedger {
    pub(crate) fn record_output_staged(
        &self,
        token: &ReservationToken,
        relative_path: &str,
        identity: OutputDirectoryIdentity,
    ) -> Result<(), RoutineError> {
        self.record_output_transition(token, relative_path, identity, false)
    }

    pub(crate) fn record_output_component(
        &self,
        token: &ReservationToken,
        relative_path: &str,
        identity: OutputDirectoryIdentity,
    ) -> Result<(), RoutineError> {
        self.record_output_transition(token, relative_path, identity, true)
    }

    pub(crate) fn reconcile_output_ambiguity(
        &self,
        token: &ReservationToken,
        ambiguity: &OutputStageAmbiguity,
    ) -> Result<(), RoutineError> {
        if token.reuse_only || token.recovery_for.is_none() {
            return Err(error(
                "routine-production-output-ambiguity-recovery-required",
            ));
        }
        self.with_payload_conditional(|payload, _tick| {
            let record = exact_record_mut(payload, token)?;
            if record.output_journal != token.output_journal {
                return Err(error("routine-production-output-ambiguity-binding-invalid"));
            }
            let component = record
                .output_journal
                .components
                .iter()
                .find(|component| component.relative_path == ambiguity.relative_path)
                .ok_or_else(|| error("routine-production-output-ambiguity-binding-invalid"))?;
            if component.preexisting.is_some()
                || component.creation_nonce.as_deref() != Some(&ambiguity.creation_nonce)
                || component.staged.is_some()
                || component.provisioned.is_some()
            {
                return Err(error("routine-production-output-ambiguity-binding-invalid"));
            }
            match record.state {
                AttemptState::Reserved => {
                    record.state = AttemptState::Incomplete;
                    Ok(((), true))
                }
                AttemptState::Incomplete => Ok(((), false)),
                AttemptState::Started
                | AttemptState::Complete
                | AttemptState::Failed
                | AttemptState::Cancelled => Err(error(
                    "routine-production-output-ambiguity-transition-invalid",
                )),
            }
        })
    }

    fn record_output_transition(
        &self,
        token: &ReservationToken,
        relative_path: &str,
        identity: OutputDirectoryIdentity,
        final_record: bool,
    ) -> Result<(), RoutineError> {
        if token.reuse_only || validate_output_journal(&token.output_journal).is_err() {
            return Err(error(
                "routine-production-output-journal-transition-invalid",
            ));
        }
        self.with_payload(true, |payload, _tick| {
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
                Some(_) => return Ok(()),
                None => *destination = Some(identity),
            }
            Ok(())
        })
    }
}
