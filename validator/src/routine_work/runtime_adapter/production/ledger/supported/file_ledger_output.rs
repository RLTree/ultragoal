use super::*;

impl FileLedger {
    pub(crate) fn record_output_component(
        &self,
        token: &ReservationToken,
        relative_path: &str,
        identity: OutputDirectoryIdentity,
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
            match component.provisioned {
                Some(observed) if observed != identity => {
                    return Err(error("routine-production-output-identity-changed"));
                }
                Some(_) => return Ok(()),
                None => component.provisioned = Some(identity),
            }
            Ok(())
        })
    }
}
