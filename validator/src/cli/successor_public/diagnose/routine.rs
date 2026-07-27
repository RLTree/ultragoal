use super::*;
use serde_json::json;

pub(super) fn diagnose(
    root: &Path,
    read_context: &LiveContext,
    invocation: &ParsedInvocation,
    home: Option<&Path>,
    fallback: Option<RuntimeOutcome>,
) -> RuntimeOutcome {
    let request = match super::request(invocation) {
        Ok(request) => request,
        Err(()) => return super::invalid_invocation(),
    };
    let binding = match super::super::routine::current_diagnosis_binding(
        root,
        request.target,
        read_context,
    ) {
        Ok(Some(binding)) => binding,
        Ok(None) => return fallback.unwrap_or_else(super::super::inventory_unavailable),
        Err(()) => return super::diagnosis_unavailable(),
    };
    let checkpoint = match home {
        Some(home) => match super::super::routine::current_checkpoint(home, &binding) {
            Ok(checkpoint) => checkpoint,
            Err(()) => return super::diagnosis_unavailable(),
        },
        None => return super::diagnosis_unavailable(),
    };
    if let Some(finding) = request.finding {
        if checkpoint
            .as_ref()
            .and_then(|checkpoint| checkpoint.finding_id.as_deref())
            != Some(finding)
        {
            return super::finding_not_present();
        }
    }
    let disposition = disposition(checkpoint.as_ref());
    let machine = serde_json::to_vec(&json!({
        "schema_version": "RoutineDiagnosis-v1",
        "context_id": binding.context().context_id(),
        "candidate_id": binding.plan().binding().candidate_id(),
        "repository_root_id": binding.plan().binding().root_id(),
        "plan_id": binding.plan().plan_id(),
        "snapshot_id": binding.snapshot().snapshot_id(),
        "status": disposition.status,
        "effect_state": disposition.effect_state,
        "recovery": disposition.recovery,
        "repeat_use": disposition.repeat_use,
        "checkpoint": checkpoint.as_ref().map(|checkpoint| json!({
            "state": checkpoint.state,
            "terminal_outcome": checkpoint.terminal_outcome,
            "event_id": checkpoint.event_id,
            "event_status": checkpoint.event_status,
            "event_transition": checkpoint.event_transition,
            "finding_id": checkpoint.finding_id,
        })),
        "claim_effect": "none",
        "support_limit": "routine-bound diagnosis only; no ProductState, Product Fitness, readiness, or release claim"
    }));
    if binding.context().revalidate().is_err() || read_context.revalidate().is_err() {
        return super::stale_context();
    }
    match machine {
        Ok(machine) if super::super::public_output_allowed(machine.len()) => {
            RuntimeOutcome::payload(
                disposition.exit,
                machine,
                format!(
                    "routine diagnosis status={} effect_state={} claim_effect=none",
                    disposition.status, disposition.effect_state
                ),
            )
        }
        _ => super::diagnosis_unavailable(),
    }
}

pub(super) struct Disposition {
    pub(super) status: &'static str,
    pub(super) effect_state: &'static str,
    pub(super) recovery: &'static str,
    pub(super) repeat_use: &'static str,
    pub(super) exit: ExitClass,
}

pub(super) fn disposition(
    checkpoint: Option<&super::super::routine::RoutineCheckpointProjection>,
) -> Disposition {
    let Some(checkpoint) = checkpoint else {
        return Disposition {
            status: "no_record",
            effect_state: "no_effect",
            recovery: "run the current candidate-bound routine command",
            repeat_use: "not_available",
            exit: ExitClass::ActionableFinding,
        };
    };
    match checkpoint.state.as_str() {
        "reserved" => Disposition {
            status: "outcome_unknown",
            effect_state: "ambiguous",
            recovery: "use only the exact continuation emitted by the interrupted routine",
            repeat_use: "withheld",
            exit: ExitClass::ActionableFinding,
        },
        "reconciled" => Disposition {
            status: "no_effect_reconciled",
            effect_state: "no_effect",
            recovery: "rerun the current candidate-bound routine command",
            repeat_use: "honest_rerun",
            exit: ExitClass::ActionableFinding,
        },
        "ambiguous" => Disposition {
            status: "ambiguous",
            effect_state: "ambiguous",
            recovery: "preserve custody and require the existing root recovery authority",
            repeat_use: "withheld",
            exit: ExitClass::ActionableFinding,
        },
        "terminal-event-pending" if checkpoint.terminal_outcome.as_deref() == Some("complete") => {
            Disposition {
                status: "settlement_pending",
                effect_state: "committed",
                recovery: "rerun routine to join the terminal event without repeating the effect",
                repeat_use: "withheld_until_settlement",
                exit: ExitClass::ActionableFinding,
            }
        }
        "terminal-event-joined" if checkpoint.terminal_outcome.as_deref() == Some("complete") => {
            Disposition {
                status: "complete",
                effect_state: "committed",
                recovery: "none",
                repeat_use: "safe_reuse",
                exit: ExitClass::Success,
            }
        }
        "terminal-event-pending" | "terminal-event-joined" => Disposition {
            status: "terminal_failure",
            effect_state: "committed",
            recovery: "inspect the terminal outcome before one candidate-bound rerun",
            repeat_use: "honest_rerun",
            exit: ExitClass::ActionableFinding,
        },
        _ => Disposition {
            status: "invalid",
            effect_state: "unknown",
            recovery: "repair the sealed routine checkpoint before retrying",
            repeat_use: "withheld",
            exit: ExitClass::ActionableFinding,
        },
    }
}

#[cfg(test)]
#[path = "routine_tests.rs"]
mod tests;
