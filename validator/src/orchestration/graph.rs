use super::model::{MAX_COLLECTION, validate_identifier};
use super::{OrchestrationError, WorkPackage};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WorkProgress {
    pub completed_nodes: BTreeSet<String>,
    pub active_nodes: BTreeSet<String>,
    pub available_tools: BTreeSet<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanBlock {
    DependencyPending,
    RequiredToolMissing,
    Active,
    ScopeConflict,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub ready: Vec<String>,
    pub blocked: BTreeMap<String, PlanBlock>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkGraph {
    packages: BTreeMap<String, WorkPackage>,
}

impl WorkGraph {
    /// Derives a generic work graph from current product-semantic observations.
    /// Node names are data; the kernel contains no historical lane registry.
    pub fn derive(packages: Vec<WorkPackage>) -> Result<Self, OrchestrationError> {
        if packages.is_empty() || packages.len() > MAX_COLLECTION {
            return Err(OrchestrationError::ResourceLimit);
        }
        let mut by_id = BTreeMap::new();
        for package in packages {
            package.validate()?;
            let node_id = package.node_id.clone();
            if by_id.insert(node_id, package).is_some() {
                return Err(OrchestrationError::DuplicateNode);
            }
        }
        for package in by_id.values() {
            if package
                .dependencies
                .iter()
                .any(|dependency| !by_id.contains_key(dependency))
            {
                return Err(OrchestrationError::UnknownNode);
            }
        }
        if contains_cycle(&by_id) {
            return Err(OrchestrationError::DependencyCycle);
        }
        Ok(Self { packages: by_id })
    }

    pub fn package(&self, node_id: &str) -> Option<&WorkPackage> {
        self.packages.get(node_id)
    }

    pub fn contains(&self, node_id: &str) -> bool {
        self.packages.contains_key(node_id)
    }

    pub fn plan(&self, progress: &WorkProgress) -> Result<Plan, OrchestrationError> {
        if progress
            .completed_nodes
            .iter()
            .chain(progress.active_nodes.iter())
            .any(|node| !self.packages.contains_key(node))
        {
            return Err(OrchestrationError::UnknownNode);
        }
        for tool in &progress.available_tools {
            validate_identifier(tool)?;
        }
        let mut plan = Plan::default();
        let mut selected: Vec<&WorkPackage> = Vec::new();
        for (node_id, package) in &self.packages {
            if progress.completed_nodes.contains(node_id) {
                continue;
            }
            if progress.active_nodes.contains(node_id) {
                plan.blocked.insert(node_id.clone(), PlanBlock::Active);
            } else if !package.dependencies.is_subset(&progress.completed_nodes) {
                plan.blocked
                    .insert(node_id.clone(), PlanBlock::DependencyPending);
            } else if !package.required_tools.is_subset(&progress.available_tools) {
                plan.blocked
                    .insert(node_id.clone(), PlanBlock::RequiredToolMissing);
            } else if selected.iter().any(|other| {
                package.safety_class == super::SafetyClass::RootSerialized
                    || other.safety_class == super::SafetyClass::RootSerialized
                    || package.owned_scope.conflicts(&other.owned_scope)
            }) {
                plan.blocked
                    .insert(node_id.clone(), PlanBlock::ScopeConflict);
            } else {
                selected.push(package);
                plan.ready.push(node_id.clone());
            }
        }
        Ok(plan)
    }

    pub fn dependency_closed(
        &self,
        completed: &BTreeSet<String>,
    ) -> Result<bool, OrchestrationError> {
        if completed
            .iter()
            .any(|node| !self.packages.contains_key(node))
        {
            return Err(OrchestrationError::UnknownNode);
        }
        Ok(completed.iter().all(|node| {
            self.packages
                .get(node)
                .is_some_and(|package| package.dependencies.is_subset(completed))
        }))
    }

    pub fn nodes(&self) -> impl Iterator<Item = &str> {
        self.packages.keys().map(String::as_str)
    }
}

fn contains_cycle(packages: &BTreeMap<String, WorkPackage>) -> bool {
    fn visit(
        node: &str,
        packages: &BTreeMap<String, WorkPackage>,
        temporary: &mut BTreeSet<String>,
        permanent: &mut BTreeSet<String>,
    ) -> bool {
        if permanent.contains(node) {
            return false;
        }
        if !temporary.insert(node.to_owned()) {
            return true;
        }
        let cycle = packages[node]
            .dependencies
            .iter()
            .any(|dependency| visit(dependency, packages, temporary, permanent));
        temporary.remove(node);
        permanent.insert(node.to_owned());
        cycle
    }

    let mut temporary = BTreeSet::new();
    let mut permanent = BTreeSet::new();
    packages
        .keys()
        .any(|node| visit(node, packages, &mut temporary, &mut permanent))
}
