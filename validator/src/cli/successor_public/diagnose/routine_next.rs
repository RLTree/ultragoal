use super::routine_projection::disposition;
use super::*;
use serde_json::json;

#[cfg(test)]
use super::super::routine::RoutineCheckpointProjection;

pub(super) fn project(
    root: &Path,
    read_context: &LiveContext,
    invocation: &ParsedInvocation,
    home: Option<&Path>,
    fallback: RuntimeOutcome,
) -> RuntimeOutcome {
    if invocation.command != SuccessorCommand::Next
        || invocation.effect != EffectClass::Read
        || !invocation.arguments.is_empty()
    {
        return fallback;
    }
    let Some(home) = home else {
        return fallback;
    };
    let binding = match super::super::routine::current_diagnosis_binding(root, None, read_context) {
        Ok(Some(binding)) => binding,
        Ok(None) | Err(()) => return fallback,
    };
    let checkpoint = match super::super::routine::current_checkpoint(home, &binding) {
        Ok(checkpoint) => checkpoint,
        Err(()) => return fallback,
    };
    if read_context.revalidate().is_err() || binding.context().revalidate().is_err() {
        return fallback;
    }
    let disposition = disposition(checkpoint.as_ref());
    let (action, command, effect) = action_for(checkpoint.as_ref());
    let reinvoke = root_bound_reinvoke(binding.plan().binding().root_id(), action);
    let machine = serde_json::to_vec(&json!({
        "schema_version": "RoutineNext-v1",
        "context_id": binding.context().context_id(),
        "candidate_id": binding.plan().binding().candidate_id(),
        "repository_root_id": binding.plan().binding().root_id(),
        "plan_id": binding.plan().plan_id(),
        "snapshot_id": binding.snapshot().snapshot_id(),
        "status": disposition.status,
        "effect_state": disposition.effect_state,
        "repeat_use": disposition.repeat_use,
        "next_action": {
            "action": action,
            "command": command,
            "effect": effect,
            "recovery_note": disposition.recovery,
            "reinvoke": reinvoke,
        },
        "claim_effect": "none",
        "support_limit": "routine-bound navigation only; no ProductState, Product Fitness, readiness, or release claim"
    }));
    match machine {
        Ok(machine) if super::super::public_output_allowed(machine.len()) => {
            RuntimeOutcome::payload(
                disposition.exit,
                machine,
                format!(
                    "routine next status={} action={} claim_effect=none",
                    disposition.status, action
                ),
            )
        }
        _ => fallback,
    }
}

/// A public next action is descriptive until the caller supplies the root that
/// was bound by the original invocation.  It intentionally carries no path or
/// cwd fallback, so copying it into another shell cannot retarget a write.
fn root_bound_reinvoke(root_id: &str, action: &str) -> serde_json::Value {
    let arguments = match action {
        "run_current_routine" | "settle_current_routine" | "reuse_current_routine" => {
            vec!["--json", "check", "routine"]
        }
        "diagnose_before_effect" => vec!["--json", "diagnose"],
        _ => Vec::new(),
    };
    json!({
        "kind": "root-bound",
        "root_binding": {
            "id": root_id,
            "source": "original-invocation",
            "required": true,
            "cwd_fallback": "refuse"
        },
        "arguments": arguments,
    })
}

fn action_for(
    checkpoint: Option<&super::super::routine::RoutineCheckpointProjection>,
) -> (&'static str, &'static str, &'static str) {
    let Some(checkpoint) = checkpoint else {
        return (
            "run_current_routine",
            "ultragoal --json check routine",
            "workspace_write",
        );
    };
    match checkpoint.state.as_str() {
        "reserved" | "reconciled" | "ambiguous" | "terminal-event-joined"
            if checkpoint.terminal_outcome.as_deref() != Some("complete") =>
        {
            (
                "diagnose_before_effect",
                "ultragoal --json diagnose",
                "read",
            )
        }
        "terminal-event-pending" if checkpoint.terminal_outcome.as_deref() == Some("complete") => (
            "settle_current_routine",
            "ultragoal --json check routine",
            "workspace_write",
        ),
        "terminal-event-joined" => (
            "reuse_current_routine",
            "ultragoal --json check routine",
            "workspace_write",
        ),
        _ => (
            "diagnose_before_effect",
            "ultragoal --json diagnose",
            "read",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn checkpoint(state: &str, outcome: Option<&str>) -> RoutineCheckpointProjection {
        RoutineCheckpointProjection {
            state: state.to_owned(),
            terminal_outcome: outcome.map(str::to_owned),
            event_id: "event-id".to_owned(),
            event_status: "status".to_owned(),
            event_transition: "transition".to_owned(),
            finding_id: None,
        }
    }

    #[test]
    fn routine_next_never_mints_or_reveals_recovery_authority() {
        for checkpoint in [
            None,
            Some(checkpoint("reserved", None)),
            Some(checkpoint("reconciled", None)),
            Some(checkpoint("ambiguous", Some("ambiguous"))),
            Some(checkpoint("terminal-event-pending", Some("complete"))),
            Some(checkpoint("terminal-event-joined", Some("complete"))),
            Some(checkpoint("terminal-event-joined", Some("failed"))),
        ] {
            let (action, command, _) = action_for(checkpoint.as_ref());
            assert!(!action.contains("continuation"));
            assert!(!command.contains("continuation"));
        }
    }

    #[test]
    fn routine_next_reuses_only_completed_custody() {
        assert_eq!(
            action_for(Some(&checkpoint("terminal-event-joined", Some("complete")))).0,
            "reuse_current_routine"
        );
        assert_eq!(
            action_for(Some(&checkpoint("reserved", None))).0,
            "diagnose_before_effect"
        );
    }

    #[test]
    fn routine_next_settles_only_a_completed_terminal_event() {
        assert_eq!(
            action_for(Some(&checkpoint(
                "terminal-event-pending",
                Some("complete")
            )))
            .0,
            "settle_current_routine"
        );
        for outcome in ["failed", "cancelled", "incomplete"] {
            let (action, command, effect) =
                action_for(Some(&checkpoint("terminal-event-pending", Some(outcome))));
            assert_eq!(action, "diagnose_before_effect", "{outcome}");
            assert_eq!(command, "ultragoal --json diagnose", "{outcome}");
            assert_eq!(effect, "read", "{outcome}");
        }
    }

    #[test]
    fn copied_next_action_cannot_silently_retarget_cwd() {
        let root_id = "root-id-for-test";
        let reinvoke = root_bound_reinvoke(root_id, "run_current_routine");
        assert_eq!(reinvoke["kind"], "root-bound");
        assert_eq!(reinvoke["root_binding"]["id"], root_id);
        assert_eq!(reinvoke["root_binding"]["source"], "original-invocation");
        assert_eq!(reinvoke["root_binding"]["required"], true);
        assert_eq!(reinvoke["root_binding"]["cwd_fallback"], "refuse");
        assert_eq!(reinvoke["arguments"], json!(["--json", "check", "routine"]));
        let encoded = serde_json::to_string(&reinvoke).expect("reinvoke JSON");
        assert!(!encoded.contains("/"));
    }
}
