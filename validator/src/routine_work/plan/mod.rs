mod definition;
mod order;
mod selection;

use serde::Serialize;

use crate::context::LiveContext;

#[cfg(test)]
use super::authority::test_authority_checkpoint;
use super::authority::{ensure_unchanged, require_current_binding};
use super::digest::{digest_of, framed};
use super::{CheckNode, DirtySnapshot, ImpactGraph, RoutineBinding, RoutineError, RoutineErrorId};
use order::topological_order;
use selection::Selection;

pub use definition::{
    AffectedSet, CoverageDimensions, PlanMode, PlanRequest, PlannedCheck, RoutinePlan,
    SelectionReason,
};

#[derive(Serialize)]
struct PlanPayload<'a> {
    binding_id: &'a str,
    graph_id: &'a str,
    snapshot_id: &'a str,
    affected: &'a AffectedSet,
    checks: &'a [PlannedCheck],
}

pub fn plan_routine(
    context: &LiveContext,
    graph: &ImpactGraph,
    snapshot: &DirtySnapshot,
    request: PlanRequest,
) -> Result<RoutinePlan, RoutineError> {
    let binding =
        require_current_binding(context, snapshot.binding(), "snapshot-binding-is-stale")?;
    snapshot.require_complete_capture()?;
    #[cfg(test)]
    test_authority_checkpoint();
    let mut selection = Selection::from_changes(graph, snapshot)?;
    selection.apply_request(graph, request)?;
    selection.close(graph);
    let order = topological_order(graph, selection.selected())?;
    let (checks, fallback_count) = build_checks(&binding, graph, snapshot, &order, &selection)?;
    let mode = if checks.is_empty() {
        PlanMode::NoOp
    } else if selection.is_strict() {
        PlanMode::Strict
    } else {
        PlanMode::Fast
    };
    let affected = selection.into_affected_set(mode, order, fallback_count);
    let plan_id = digest_of(&PlanPayload {
        binding_id: binding.binding_id(),
        graph_id: graph.graph_id(),
        snapshot_id: snapshot.snapshot_id(),
        affected: &affected,
        checks: &checks,
    })?;
    ensure_unchanged(context, &binding, "routine-binding-changed-during-planning")?;
    Ok(RoutinePlan::new(
        plan_id,
        binding,
        graph.graph_id().to_owned(),
        snapshot.snapshot_id().to_owned(),
        affected,
        checks,
    ))
}

fn build_checks(
    binding: &RoutineBinding,
    graph: &ImpactGraph,
    snapshot: &DirtySnapshot,
    order: &[String],
    selection: &Selection,
) -> Result<(Vec<PlannedCheck>, usize), RoutineError> {
    let mut checks = Vec::with_capacity(order.len());
    let mut fallback_count = 0;
    for node_id in order {
        let node = &graph.nodes()[node_id];
        let (tool, identity, fallback) = choose_tool(binding, node)?;
        fallback_count += usize::from(fallback);
        checks.push(PlannedCheck::new(
            node_id.clone(),
            node.depends_on()
                .intersection(selection.selected())
                .cloned()
                .collect(),
            tool,
            identity,
            fallback,
            framed(&[
                snapshot.snapshot_id().as_bytes(),
                graph.graph_id().as_bytes(),
                node_id.as_bytes(),
            ]),
        ));
    }
    Ok((checks, fallback_count))
}

fn choose_tool(
    binding: &RoutineBinding,
    node: &CheckNode,
) -> Result<(String, String, bool), RoutineError> {
    if let Some(tool) = binding
        .tool(node.runner().primary())
        .filter(|tool| tool.available())
    {
        return Ok((
            node.runner().primary().to_owned(),
            tool.identity_sha256().to_owned(),
            false,
        ));
    }
    if let Some(name) = node.runner().fallback()
        && let Some(tool) = binding.tool(name).filter(|tool| tool.available())
    {
        return Ok((name.to_owned(), tool.identity_sha256().to_owned(), true));
    }
    Err(RoutineError::new(
        RoutineErrorId::CapabilityUnavailable,
        "node-runner-and-fallback-unavailable",
        None,
    ))
}
