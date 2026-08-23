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
        predecessor_continuations: &[String],
        attempt_grant: &str,
        authenticated_ledger_head: &str,
        state: &str,
        terminal_outcome: Option<&str>,
        allow_stale_head: bool,
    ) -> Result<(), RoutineError> {
        if !admissible_target(target, state, terminal_outcome, allow_stale_head)
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
                || record.predecessor_continuations != predecessor_continuations
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
                    AttemptState::RolledBack
                        | AttemptState::Complete
                        | AttemptState::Failed
                        | AttemptState::Cancelled
                        | AttemptState::Incomplete
                )
            {
                return Err(error("routine-production-checkpoint-alias-invalid"));
            }
            Ok(((), false))
        })?
        .into_result()
    }
}

pub(super) fn admissible_target(
    target: &Path,
    state: &str,
    terminal_outcome: Option<&str>,
    allow_stale_head: bool,
) -> bool {
    if !target.is_absolute() || target.to_str().is_none() {
        return false;
    }
    if std::fs::canonicalize(target).ok().as_deref() == Some(target) {
        return true;
    }
    let settled_history = allow_stale_head
        && matches!(state, "terminal-event-pending" | "terminal-event-joined")
        && matches!(
            terminal_outcome,
            Some("complete" | "failed" | "cancelled" | "incomplete")
        );
    settled_history && missing_target_has_canonical_ancestry(target)
}

fn missing_target_has_canonical_ancestry(target: &Path) -> bool {
    if target.components().any(|component| {
        !matches!(
            component,
            std::path::Component::RootDir | std::path::Component::Normal(_)
        )
    }) || !matches!(
        std::fs::symlink_metadata(target),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound
    ) {
        return false;
    }
    let mut ancestor = target.parent();
    while let Some(path) = ancestor {
        match std::fs::symlink_metadata(path) {
            Ok(metadata) => {
                return metadata.is_dir()
                    && std::fs::canonicalize(path).ok().as_deref() == Some(path);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                ancestor = path.parent();
            }
            Err(_) => return false,
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::{admissible_target, missing_target_has_canonical_ancestry};
    use std::path::Path;

    #[test]
    fn only_settled_stale_history_may_reference_a_missing_canonical_target() {
        let target = Path::new("/private/tmp/ultragoal-settled-history-missing-target");
        assert!(missing_target_has_canonical_ancestry(target));
        assert!(admissible_target(
            target,
            "terminal-event-joined",
            Some("complete"),
            true,
        ));
        assert!(!admissible_target(
            target,
            "terminal-event-joined",
            Some("complete"),
            false,
        ));
        assert!(!admissible_target(
            target,
            "ambiguous",
            Some("ambiguous"),
            true,
        ));
        assert!(!missing_target_has_canonical_ancestry(Path::new(
            "/private/tmp/../tmp/ultragoal-settled-history-missing-target",
        )));
    }
}

pub(super) fn valid_continuation(value: &str) -> bool {
    value.strip_prefix("routine-cont-").is_some_and(valid)
}

pub(super) fn continuation_for(record: &ProtocolRecord) -> String {
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
