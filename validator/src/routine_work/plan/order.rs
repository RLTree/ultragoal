use std::collections::BTreeSet;

use crate::routine_work::{ImpactGraph, RoutineError, RoutineErrorId};

pub(super) fn topological_order(
    graph: &ImpactGraph,
    selected: &BTreeSet<String>,
) -> Result<Vec<String>, RoutineError> {
    let mut remaining = selected.clone();
    let mut emitted = BTreeSet::new();
    let mut order = Vec::with_capacity(selected.len());
    while !remaining.is_empty() {
        let ready = remaining
            .iter()
            .filter(|node| {
                graph.nodes()[*node].depends_on().iter().all(|dependency| {
                    !selected.contains(dependency) || emitted.contains(dependency)
                })
            })
            .cloned()
            .collect::<Vec<_>>();
        if ready.is_empty() {
            return Err(RoutineError::new(
                RoutineErrorId::InvalidRegistry,
                "selected-dependency-cycle",
                None,
            ));
        }
        for node in ready {
            remaining.remove(&node);
            emitted.insert(node.clone());
            order.push(node);
        }
    }
    Ok(order)
}
