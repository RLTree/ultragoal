use std::collections::{BTreeMap, BTreeSet};

use super::model::{AffectedSet, CoverageDimensions, PlanMode, PlanRequest, SelectionReason};
use crate::routine_work::{
    ChangeKind, CheckClass, DirtySnapshot, ImpactGraph, RoutineError, RoutineErrorId,
};

pub(super) struct Selection {
    selected: BTreeSet<String>,
    direct: BTreeSet<String>,
    reasons: BTreeMap<String, BTreeSet<SelectionReason>>,
    changed_paths: usize,
    direct_count: usize,
    downstream_count: usize,
    dependency_count: usize,
    unknown_paths: usize,
    conflicts: usize,
    strict: bool,
    strict_claim: bool,
}

impl Selection {
    pub(super) fn from_changes(
        graph: &ImpactGraph,
        snapshot: &DirtySnapshot,
    ) -> Result<Self, RoutineError> {
        let mut selection = Self {
            selected: BTreeSet::new(),
            direct: BTreeSet::new(),
            reasons: BTreeMap::new(),
            changed_paths: 0,
            direct_count: 0,
            downstream_count: 0,
            dependency_count: 0,
            unknown_paths: 0,
            conflicts: 0,
            strict: false,
            strict_claim: false,
        };
        for change in snapshot.changes() {
            if change.kind() == ChangeKind::Conflict {
                selection.conflicts += 1;
                selection.strict = true;
            }
            let paths = [Some(change.path()), change.previous_path()];
            for path in paths.into_iter().flatten() {
                selection.changed_paths += 1;
                let matches = graph
                    .routes()
                    .iter()
                    .filter(|route| route.matcher().matches(path))
                    .collect::<Vec<_>>();
                if matches.is_empty() {
                    selection.unknown_paths += 1;
                    selection.strict = true;
                }
                for route in matches {
                    selection.strict |= route.requires_strict();
                    for node in route.node_ids() {
                        selection.direct.insert(node.clone());
                        selection.select(node, SelectionReason::DirectImpact);
                    }
                }
            }
        }
        selection.direct_count = selection.direct.len();
        if selection.unknown_paths > 0 || selection.conflicts > 0 {
            let reason = if selection.conflicts > 0 {
                SelectionReason::ConflictExpansion
            } else {
                SelectionReason::UnknownImpactExpansion
            };
            for node in graph.nodes().keys() {
                selection.select(node, reason);
            }
        }
        Ok(selection)
    }

    pub(super) fn apply_request(
        &mut self,
        graph: &ImpactGraph,
        request: PlanRequest,
    ) -> Result<(), RoutineError> {
        match request {
            PlanRequest::Routine => {}
            PlanRequest::RoutineWithNodes(nodes) => {
                let mut seen = BTreeSet::new();
                for node in nodes {
                    if !seen.insert(node.clone()) {
                        return Err(RoutineError::new(
                            RoutineErrorId::InvalidRequest,
                            "explicit-node-duplicated",
                            None,
                        ));
                    }
                    if !graph.nodes().contains_key(&node) {
                        return Err(unknown_error("explicit-node-unknown"));
                    }
                    self.select(&node, SelectionReason::ExplicitRequest);
                }
            }
            PlanRequest::StrictClaim(claim_id) => {
                let boundary = graph
                    .claim(&claim_id)
                    .ok_or_else(|| unknown_error("strict-claim-unknown"))?;
                self.strict = true;
                self.strict_claim = true;
                for node in boundary.node_ids() {
                    self.select(node, SelectionReason::ClaimBoundary);
                }
            }
        }
        Ok(())
    }

    pub(super) fn close(&mut self, graph: &ImpactGraph) {
        if !self.strict_claim && self.unknown_paths == 0 && self.conflicts == 0 {
            self.close_downstream(graph);
        }
        self.downstream_count = count_reason(&self.reasons, SelectionReason::DownstreamImpact);
        self.close_dependencies(graph);
        self.dependency_count = count_reason(&self.reasons, SelectionReason::Dependency);
    }

    fn close_downstream(&mut self, graph: &ImpactGraph) {
        loop {
            let before = self.selected.len();
            for (node_id, node) in graph.nodes() {
                if node.class() != CheckClass::ClaimBoundaryOnly
                    && node
                        .depends_on()
                        .iter()
                        .any(|dependency| self.direct.contains(dependency))
                {
                    self.select(node_id, SelectionReason::DownstreamImpact);
                }
            }
            self.direct.extend(self.selected.iter().cloned());
            if self.selected.len() == before {
                break;
            }
        }
    }

    fn close_dependencies(&mut self, graph: &ImpactGraph) {
        loop {
            let additions = self
                .selected
                .iter()
                .flat_map(|node| graph.nodes()[node].depends_on())
                .filter(|dependency| !self.selected.contains(*dependency))
                .cloned()
                .collect::<BTreeSet<_>>();
            if additions.is_empty() {
                break;
            }
            for dependency in additions {
                self.select(&dependency, SelectionReason::Dependency);
            }
        }
    }

    fn select(&mut self, node: &str, reason: SelectionReason) {
        self.selected.insert(node.to_owned());
        self.reasons
            .entry(node.to_owned())
            .or_default()
            .insert(reason);
    }

    pub(super) fn selected(&self) -> &BTreeSet<String> {
        &self.selected
    }

    pub(super) fn is_strict(&self) -> bool {
        self.strict
    }

    pub(super) fn into_affected_set(
        self,
        mode: PlanMode,
        node_ids: Vec<String>,
        fallback_tool_count: usize,
    ) -> AffectedSet {
        AffectedSet {
            mode,
            node_ids,
            reasons: self.reasons,
            coverage: CoverageDimensions {
                changed_path_count: self.changed_paths,
                direct_node_count: self.direct_count,
                downstream_expansion_count: self.downstream_count,
                dependency_expansion_count: self.dependency_count,
                unknown_path_count: self.unknown_paths,
                conflict_count: self.conflicts,
                fallback_tool_count,
                strict_boundary: self.strict,
            },
        }
    }
}

fn count_reason(
    reasons: &BTreeMap<String, BTreeSet<SelectionReason>>,
    expected: SelectionReason,
) -> usize {
    reasons
        .values()
        .filter(|values| values.contains(&expected))
        .count()
}

fn unknown_error(cause: &'static str) -> RoutineError {
    RoutineError::new(RoutineErrorId::UnknownRegistryRow, cause, None)
}
