use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

use crate::routine_work::{RoutineBinding, RoutineError};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PlanMode {
    NoOp,
    Fast,
    Strict,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SelectionReason {
    DirectImpact,
    DownstreamImpact,
    Dependency,
    ExplicitRequest,
    ClaimBoundary,
    UnknownImpactExpansion,
    ConflictExpansion,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub enum PlanRequest {
    Routine,
    RoutineWithNodes(Vec<String>),
    StrictClaim(String),
}

impl PlanRequest {
    pub fn routine() -> Self {
        Self::Routine
    }

    pub fn routine_with_nodes(nodes: impl IntoIterator<Item = String>) -> Self {
        Self::RoutineWithNodes(nodes.into_iter().collect())
    }

    pub fn strict_claim(claim_id: impl Into<String>) -> Self {
        Self::StrictClaim(claim_id.into())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CoverageDimensions {
    pub(super) changed_path_count: usize,
    pub(super) direct_node_count: usize,
    pub(super) downstream_expansion_count: usize,
    pub(super) dependency_expansion_count: usize,
    pub(super) unknown_path_count: usize,
    pub(super) conflict_count: usize,
    pub(super) fallback_tool_count: usize,
    pub(super) strict_boundary: bool,
}

impl CoverageDimensions {
    pub fn changed_path_count(&self) -> usize {
        self.changed_path_count
    }

    pub fn direct_node_count(&self) -> usize {
        self.direct_node_count
    }

    pub fn downstream_expansion_count(&self) -> usize {
        self.downstream_expansion_count
    }

    pub fn dependency_expansion_count(&self) -> usize {
        self.dependency_expansion_count
    }

    pub fn unknown_path_count(&self) -> usize {
        self.unknown_path_count
    }

    pub fn conflict_count(&self) -> usize {
        self.conflict_count
    }

    pub fn fallback_tool_count(&self) -> usize {
        self.fallback_tool_count
    }

    pub fn strict_boundary(&self) -> bool {
        self.strict_boundary
    }

    pub(crate) fn identity(&self) -> Result<String, RoutineError> {
        crate::routine_work::digest::digest_of(self)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AffectedSet {
    pub(super) mode: PlanMode,
    pub(super) node_ids: Vec<String>,
    pub(super) reasons: BTreeMap<String, BTreeSet<SelectionReason>>,
    pub(super) coverage: CoverageDimensions,
}

impl AffectedSet {
    pub fn mode(&self) -> PlanMode {
        self.mode
    }

    pub fn node_ids(&self) -> &[String] {
        &self.node_ids
    }

    pub fn reasons(&self, node_id: &str) -> Option<&BTreeSet<SelectionReason>> {
        self.reasons.get(node_id)
    }

    pub fn coverage(&self) -> &CoverageDimensions {
        &self.coverage
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PlannedCheck {
    node_id: String,
    depends_on: BTreeSet<String>,
    selected_tool: String,
    selected_tool_identity: String,
    used_fallback: bool,
    input_id: String,
}

impl PlannedCheck {
    pub(super) fn new(
        node_id: String,
        depends_on: BTreeSet<String>,
        selected_tool: String,
        selected_tool_identity: String,
        used_fallback: bool,
        input_id: String,
    ) -> Self {
        Self {
            node_id,
            depends_on,
            selected_tool,
            selected_tool_identity,
            used_fallback,
            input_id,
        }
    }

    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    pub fn depends_on(&self) -> &BTreeSet<String> {
        &self.depends_on
    }

    pub fn selected_tool(&self) -> &str {
        &self.selected_tool
    }

    pub fn used_fallback(&self) -> bool {
        self.used_fallback
    }

    pub fn input_id(&self) -> &str {
        &self.input_id
    }

    pub(crate) fn selected_tool_identity(&self) -> &str {
        &self.selected_tool_identity
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RoutinePlan {
    plan_id: String,
    binding: RoutineBinding,
    graph_id: String,
    snapshot_id: String,
    affected: AffectedSet,
    checks: Vec<PlannedCheck>,
}

impl RoutinePlan {
    pub(super) fn new(
        plan_id: String,
        binding: RoutineBinding,
        graph_id: String,
        snapshot_id: String,
        affected: AffectedSet,
        checks: Vec<PlannedCheck>,
    ) -> Self {
        Self {
            plan_id,
            binding,
            graph_id,
            snapshot_id,
            affected,
            checks,
        }
    }

    pub fn plan_id(&self) -> &str {
        &self.plan_id
    }

    pub fn binding(&self) -> &RoutineBinding {
        &self.binding
    }

    pub fn graph_id(&self) -> &str {
        &self.graph_id
    }

    pub fn snapshot_id(&self) -> &str {
        &self.snapshot_id
    }

    pub fn affected_set(&self) -> &AffectedSet {
        &self.affected
    }

    pub fn checks(&self) -> &[PlannedCheck] {
        &self.checks
    }

    pub fn check(&self, node_id: &str) -> Option<&PlannedCheck> {
        self.checks.iter().find(|check| check.node_id == node_id)
    }
}
