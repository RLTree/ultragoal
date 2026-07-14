use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

use super::validation::{
    identifier, identifiers, registry_error, unique_claims, unique_nodes, validate_dependencies,
    validate_graph_size, validate_routes,
};
use crate::routine_work::{RepoPath, RoutineError};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CheckClass {
    Routine,
    ClaimBoundaryOnly,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RunnerSpec {
    primary: String,
    fallback: Option<String>,
}

impl RunnerSpec {
    pub fn new(primary: impl Into<String>, fallback: Option<String>) -> Result<Self, RoutineError> {
        let primary = identifier(primary.into())?;
        let fallback = fallback.map(identifier).transpose()?;
        if fallback.as_ref() == Some(&primary) {
            return Err(registry_error("duplicate-runner-choice"));
        }
        Ok(Self { primary, fallback })
    }

    pub fn primary(&self) -> &str {
        &self.primary
    }

    pub fn fallback(&self) -> Option<&str> {
        self.fallback.as_deref()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CheckNode {
    pub(super) node_id: String,
    pub(super) depends_on: BTreeSet<String>,
    class: CheckClass,
    runner: RunnerSpec,
}

impl CheckNode {
    pub fn new(
        node_id: impl Into<String>,
        depends_on: impl IntoIterator<Item = String>,
        class: CheckClass,
        runner: RunnerSpec,
    ) -> Result<Self, RoutineError> {
        let node_id = identifier(node_id.into())?;
        let depends_on = identifiers(depends_on, "duplicate-dependency-row")?;
        if depends_on.len() > 1_024 {
            return Err(registry_error("node-dependency-limit-exceeded"));
        }
        if depends_on.contains(&node_id) {
            return Err(registry_error("self-dependency"));
        }
        Ok(Self {
            node_id,
            depends_on,
            class,
            runner,
        })
    }

    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    pub fn depends_on(&self) -> &BTreeSet<String> {
        &self.depends_on
    }

    pub fn class(&self) -> CheckClass {
        self.class
    }

    pub fn runner(&self) -> &RunnerSpec {
        &self.runner
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PathMatcher {
    Exact(RepoPath),
    Prefix(RepoPath),
}

impl PathMatcher {
    pub(crate) fn matches(&self, path: &RepoPath) -> bool {
        match self {
            Self::Exact(expected) => path == expected,
            Self::Prefix(prefix) => path.matches_prefix(prefix),
        }
    }

    pub(super) fn case_key(&self) -> (u8, String) {
        match self {
            Self::Exact(path) => (0, path.as_str().to_ascii_lowercase()),
            Self::Prefix(path) => (1, path.as_str().to_ascii_lowercase()),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PathRoute {
    pub(super) row_id: String,
    pub(super) matcher: PathMatcher,
    pub(super) node_ids: BTreeSet<String>,
    requires_strict: bool,
}

impl PathRoute {
    pub fn new(
        row_id: impl Into<String>,
        matcher: PathMatcher,
        node_ids: impl IntoIterator<Item = String>,
        requires_strict: bool,
    ) -> Result<Self, RoutineError> {
        let row_id = identifier(row_id.into())?;
        let node_ids = identifiers(node_ids, "duplicate-route-target")?;
        if node_ids.is_empty() {
            return Err(registry_error("empty-route-target"));
        }
        if node_ids.len() > 4_096 {
            return Err(registry_error("route-target-limit-exceeded"));
        }
        Ok(Self {
            row_id,
            matcher,
            node_ids,
            requires_strict,
        })
    }

    pub fn row_id(&self) -> &str {
        &self.row_id
    }

    pub fn matcher(&self) -> &PathMatcher {
        &self.matcher
    }

    pub fn node_ids(&self) -> &BTreeSet<String> {
        &self.node_ids
    }

    pub fn requires_strict(&self) -> bool {
        self.requires_strict
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ClaimBoundary {
    pub(super) claim_id: String,
    pub(super) node_ids: BTreeSet<String>,
}

impl ClaimBoundary {
    pub fn new(
        claim_id: impl Into<String>,
        node_ids: impl IntoIterator<Item = String>,
    ) -> Result<Self, RoutineError> {
        let claim_id = identifier(claim_id.into())?;
        let node_ids = identifiers(node_ids, "duplicate-claim-target")?;
        if node_ids.is_empty() {
            return Err(registry_error("empty-claim-boundary"));
        }
        if node_ids.len() > 4_096 {
            return Err(registry_error("claim-target-limit-exceeded"));
        }
        Ok(Self { claim_id, node_ids })
    }

    pub fn claim_id(&self) -> &str {
        &self.claim_id
    }

    pub fn node_ids(&self) -> &BTreeSet<String> {
        &self.node_ids
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ImpactGraph {
    graph_id: String,
    nodes: BTreeMap<String, CheckNode>,
    routes: Vec<PathRoute>,
    claims: BTreeMap<String, ClaimBoundary>,
}

#[derive(Serialize)]
struct GraphPayload<'a> {
    nodes: &'a BTreeMap<String, CheckNode>,
    routes: &'a [PathRoute],
    claims: &'a BTreeMap<String, ClaimBoundary>,
}

impl ImpactGraph {
    pub fn new(
        nodes: Vec<CheckNode>,
        mut routes: Vec<PathRoute>,
        claims: Vec<ClaimBoundary>,
    ) -> Result<Self, RoutineError> {
        validate_graph_size(nodes.len(), routes.len(), claims.len())?;
        let nodes = unique_nodes(nodes)?;
        validate_dependencies(&nodes)?;
        routes.sort_by(|left, right| {
            (&left.matcher, &left.row_id).cmp(&(&right.matcher, &right.row_id))
        });
        validate_routes(&nodes, &routes)?;
        let claims = unique_claims(&nodes, claims)?;
        let graph_id = crate::routine_work::digest::digest_of(&GraphPayload {
            nodes: &nodes,
            routes: &routes,
            claims: &claims,
        })?;
        Ok(Self {
            graph_id,
            nodes,
            routes,
            claims,
        })
    }

    pub fn graph_id(&self) -> &str {
        &self.graph_id
    }

    pub fn nodes(&self) -> &BTreeMap<String, CheckNode> {
        &self.nodes
    }

    pub fn routes(&self) -> &[PathRoute] {
        &self.routes
    }

    pub fn claim(&self, claim_id: &str) -> Option<&ClaimBoundary> {
        self.claims.get(claim_id)
    }
}
