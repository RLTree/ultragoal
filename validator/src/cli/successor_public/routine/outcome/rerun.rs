use super::*;

pub(crate) const RERUN: &str = "ultragoal --json check routine [--target <relative-repository>]";

pub(crate) enum PublicFailure {
    InvalidInvocation,
    Manifest(ManifestFailure),
    Catalog(&'static str),
    Context,
    Routine(RoutineError),
    Host(HostFailure),
    ContinuationUnavailable,
    PersistenceAfterEffect,
}

#[derive(Serialize)]
pub(crate) struct PublicOutcome<'a> {
    pub(crate) schema_version: &'static str,
    pub(crate) command: &'static str,
    pub(crate) status: &'static str,
    pub(crate) effect: &'static str,
    pub(crate) context_id: &'a str,
    pub(crate) candidate_id: &'a str,
    pub(crate) graph_id: &'a str,
    pub(crate) snapshot_id: &'a str,
    pub(crate) plan_id: &'a str,
    pub(crate) source_id: &'a str,
    pub(crate) protocol_id: Option<&'a str>,
    pub(crate) request_id: Option<&'a str>,
    pub(crate) nodes: Vec<PublicNode<'a>>,
    pub(crate) fallback_tool_count: usize,
    pub(crate) recovery_required: bool,
    pub(crate) continuation: Option<&'a str>,
    pub(crate) claim_effect: &'static str,
    pub(crate) support_limit: &'static str,
}

#[derive(Serialize)]
pub(crate) struct PublicNode<'a> {
    pub(crate) node_id: &'a str,
    pub(crate) disposition: &'static str,
    pub(crate) result_artifact_sha256: Option<&'a str>,
    pub(crate) failure_code: Option<&'a str>,
}

pub(crate) struct MediationContext<'a> {
    pub(crate) context_id: &'a str,
    pub(crate) candidate_id: &'a str,
    pub(crate) graph_id: &'a str,
    pub(crate) snapshot_id: &'a str,
    pub(crate) plan_id: &'a str,
    pub(crate) source_id: &'a str,
    pub(crate) fallback_tool_count: usize,
}

pub(crate) fn mediation(
    result: &RoutineMediationResult,
    context: MediationContext<'_>,
) -> RuntimeOutcome {
    let reused = result.status() == RoutineMediatorStatus::CompleteExecution
        && result
            .nodes()
            .iter()
            .all(|node| node.disposition() == RoutineNodeDisposition::Reused);
    let status = match result.status() {
        RoutineMediatorStatus::CompleteNoOp => "clean-no-op",
        RoutineMediatorStatus::CompleteExecution if reused => "reused",
        RoutineMediatorStatus::CompleteExecution => "executed",
        RoutineMediatorStatus::IncompleteExecution if result.recovery_marker().is_some() => {
            "interrupted-reservation"
        }
        RoutineMediatorStatus::IncompleteExecution => "incomplete",
        RoutineMediatorStatus::Cancelled => "cancelled",
    };
    let exit = match result.status() {
        RoutineMediatorStatus::CompleteNoOp | RoutineMediatorStatus::CompleteExecution => {
            ExitClass::Success
        }
        RoutineMediatorStatus::IncompleteExecution | RoutineMediatorStatus::Cancelled => {
            ExitClass::ActionableFinding
        }
    };
    let nodes = result
        .nodes()
        .iter()
        .map(|node| PublicNode {
            node_id: node.node_id(),
            disposition: disposition(node.disposition()),
            result_artifact_sha256: node.result_artifact_sha256(),
            failure_code: node.failure_code(),
        })
        .collect();
    let payload = PublicOutcome {
        schema_version: "RoutinePublicProductionOutcome-v2",
        command: "check-routine",
        status,
        effect: if result.status() == RoutineMediatorStatus::CompleteNoOp
            || result.recovery_marker().is_some()
            || reused
        {
            "none"
        } else {
            "workspace_write"
        },
        context_id: context.context_id,
        candidate_id: context.candidate_id,
        graph_id: context.graph_id,
        snapshot_id: context.snapshot_id,
        plan_id: context.plan_id,
        source_id: context.source_id,
        protocol_id: result.protocol_id(),
        request_id: result.request_id(),
        nodes,
        fallback_tool_count: context.fallback_tool_count,
        recovery_required: result.recovery_required(),
        continuation: result.continuation(),
        claim_effect: "none",
        support_limit: result.support_limit(),
    };
    match serde_json::to_vec(&payload) {
        Ok(machine) => RuntimeOutcome::payload(
            exit,
            machine,
            format!(
                "routine {status} nodes={} claim_effect=none",
                result.nodes().len()
            ),
        ),
        Err(_) => failure(PublicFailure::PersistenceAfterEffect),
    }
}

pub(crate) fn disposition(value: RoutineNodeDisposition) -> &'static str {
    match value {
        RoutineNodeDisposition::Executed => "executed",
        RoutineNodeDisposition::Reused => "reused",
        RoutineNodeDisposition::Failed => "failed",
        RoutineNodeDisposition::DependencyFailed => "dependency-failed",
        RoutineNodeDisposition::Cancelled => "cancelled",
    }
}
