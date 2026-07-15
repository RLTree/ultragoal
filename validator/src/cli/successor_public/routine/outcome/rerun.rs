use super::*;
use crate::routine_work::PRODUCTION_SUPPORT_LIMIT;

pub(crate) const RERUN: &str = "ultragoal --json check routine [--target <relative-repository>]";

pub(crate) enum PublicFailure {
    InvalidInvocation,
    Manifest(ManifestFailure),
    Catalog(&'static str),
    Context,
    Routine(RoutineError),
    Host(HostFailure),
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

#[allow(clippy::too_many_arguments)]
pub(crate) fn mediation(
    result: &RoutineMediationResult,
    context_id: &str,
    candidate_id: &str,
    graph_id: &str,
    snapshot_id: &str,
    plan_id: &str,
    source_id: &str,
    protocol_id: Option<&str>,
    fallback_tool_count: usize,
) -> RuntimeOutcome {
    let status = match result.status() {
        RoutineMediatorStatus::CompleteNoOp => "clean-no-op",
        RoutineMediatorStatus::CompleteExecution
            if result
                .nodes()
                .iter()
                .all(|node| node.disposition() == RoutineNodeDisposition::Reused) =>
        {
            "reused"
        }
        RoutineMediatorStatus::CompleteExecution => "executed",
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
        schema_version: "RoutinePublicProductionOutcome-v1",
        command: "check-routine",
        status,
        effect: if result.status() == RoutineMediatorStatus::CompleteNoOp {
            "none"
        } else {
            "workspace_write"
        },
        context_id,
        candidate_id,
        graph_id,
        snapshot_id,
        plan_id,
        source_id,
        protocol_id,
        request_id: result.request_id(),
        nodes,
        fallback_tool_count,
        recovery_required: result.recovery_marker().is_some(),
        claim_effect: "none",
        support_limit: PRODUCTION_SUPPORT_LIMIT,
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
