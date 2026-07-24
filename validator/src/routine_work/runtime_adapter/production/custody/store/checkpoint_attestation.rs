use super::*;
use std::path::Path;

impl FileLedger {
    pub(in crate::routine_work::runtime_adapter::production::custody::store) fn authenticate_public_checkpoint(
        &self,
        local: &mut LocalHead,
        target: &Path,
        context_id: &str,
        candidate_id: &str,
        plan_id: &str,
        snapshot_id: &str,
        continuation: &str,
        recovery_marker: &str,
        predecessor_continuation: Option<&str>,
        attempt_grant: &str,
        authenticated_ledger_head: &str,
        state: &str,
        terminal_outcome: Option<&str>,
        allow_stale_head: bool,
    ) -> Result<(), RoutineError> {
        if !target.is_absolute()
            || target.to_str().is_none()
            || std::fs::canonicalize(target).ok().as_deref() != Some(target)
            || !valid(context_id)
            || !valid(candidate_id)
            || !valid(plan_id)
            || !valid(snapshot_id)
            || !valid_continuation(continuation)
            || !valid(recovery_marker)
            || !valid(attempt_grant)
            || !valid(authenticated_ledger_head)
        {
            return Err(error("routine-production-checkpoint-attestation-invalid"));
        }

        self.transition_payload(local, PublicationContext::read(), |payload, _tick, head| {
            if !allow_stale_head && head != authenticated_ledger_head {
                return Err(error("routine-production-checkpoint-ledger-head-stale"));
            }
            let record = payload
                .attempts
                .get(attempt_grant)
                .ok_or_else(|| error("routine-production-checkpoint-attempt-missing"))?;
            if record.binding.context_id != context_id
                || record.binding.candidate_id != candidate_id
                || record.binding.plan_id != plan_id
                || record.binding.snapshot_id != snapshot_id
                || record.recovery_marker != recovery_marker
                || record.predecessor_continuation.as_deref() != predecessor_continuation
                || continuation_for(record) != continuation
            {
                return Err(error("routine-production-checkpoint-binding-invalid"));
            }
            let expected = expected_attempt_state(state, terminal_outcome)?;
            if record.state != expected {
                return Err(error("routine-production-checkpoint-state-invalid"));
            }
            if allow_stale_head
                && !matches!(
                    record.state,
                    AttemptState::RolledBack | AttemptState::Complete
                )
            {
                return Err(error("routine-production-checkpoint-alias-invalid"));
            }
            Ok(((), false))
        })?
        .into_result()
    }
}

fn valid_continuation(value: &str) -> bool {
    value.strip_prefix("routine-cont-").is_some_and(valid)
}

fn continuation_for(record: &ProtocolRecord) -> String {
    format!(
        "routine-cont-{}",
        crate::routine_work::digest::digest_of(&(
            "routine-public-continuation-v1",
            &record.binding.protocol_id,
            &record.grant_id,
            &record.recovery_marker,
        ))
        .unwrap_or_default()
    )
}

fn expected_attempt_state(
    state: &str,
    terminal_outcome: Option<&str>,
) -> Result<AttemptState, RoutineError> {
    match state {
        "reserved" if terminal_outcome.is_none() => Ok(AttemptState::Reserved),
        "reconciled" if terminal_outcome.is_none() => Ok(AttemptState::RolledBack),
        "ambiguous" if terminal_outcome == Some("ambiguous") => Ok(AttemptState::Ambiguous),
        "terminal-event-pending" | "terminal-event-joined" => match terminal_outcome {
            Some("complete") => Ok(AttemptState::Complete),
            Some("failed") => Ok(AttemptState::Failed),
            Some("cancelled") => Ok(AttemptState::Cancelled),
            Some("incomplete") => Ok(AttemptState::Incomplete),
            _ => Err(error("routine-production-checkpoint-outcome-invalid")),
        },
        _ => Err(error("routine-production-checkpoint-state-invalid")),
    }
}
