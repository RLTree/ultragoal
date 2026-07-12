use super::artifact::validate_records as validate_artifact_records;
use super::model::{
    MAX_COLLECTION, validate_actor_identifier, validate_digest, validate_identifier,
};
use super::scope::symbol_in_prefix;
use super::{
    ArtifactRecord, CanonicalPath, EffectClass, EffectGrant, LeaseSpec, OrchestrationError,
    RootChangeRequest, SafetyClass, WorkPackage,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const MAX_RESULT_BYTES: usize = 4 * 1024 * 1024;
pub const NO_CLAIM: &str = "This worker does not claim readiness, release, or completion.";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EffectUse {
    pub class: EffectClass,
    pub target: String,
    pub performed: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerResultV1 {
    pub worker: String,
    pub lease_id: String,
    pub context_id: String,
    pub candidate_identity: BTreeMap<String, Value>,
    pub base_state: BTreeMap<String, Value>,
    pub final_state: BTreeMap<String, Value>,
    pub touched_paths: Vec<String>,
    pub touched_semantics: Vec<String>,
    pub generated_outputs: Vec<String>,
    pub fixtures: Vec<String>,
    pub effects: Vec<EffectUse>,
    pub requirements: Vec<String>,
    pub dependency_nodes: Vec<String>,
    pub changes: Vec<BTreeMap<String, Value>>,
    pub commands_and_tests: Vec<BTreeMap<String, Value>>,
    pub artifacts: Vec<ArtifactRecord>,
    pub findings: Vec<BTreeMap<String, Value>>,
    pub unresolved_dependencies: Vec<String>,
    pub requested_root_changes: Vec<RootChangeRequest>,
    pub limitations: Vec<String>,
    pub no_claim_statement: String,
}

impl WorkerResultV1 {
    pub fn parse_json(bytes: &[u8]) -> Result<Self, OrchestrationError> {
        if bytes.is_empty() || bytes.len() > MAX_RESULT_BYTES {
            return Err(OrchestrationError::ResourceLimit);
        }
        serde_json::from_slice(bytes).map_err(|_| OrchestrationError::InvalidWorkerResult)
    }

    pub fn result_id(&self) -> Result<String, OrchestrationError> {
        let bytes =
            serde_json::to_vec(self).map_err(|_| OrchestrationError::InvalidWorkerResult)?;
        if bytes.len() > MAX_RESULT_BYTES {
            return Err(OrchestrationError::ResourceLimit);
        }
        Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
    }

    pub fn validate_for(
        &self,
        lease: &LeaseSpec,
        package: &WorkPackage,
    ) -> Result<(), OrchestrationError> {
        let _ = self.result_id()?;
        validate_actor_identifier(&self.worker)?;
        validate_identifier(&self.lease_id)?;
        validate_digest(&self.context_id)?;
        if self.worker != lease.owner.as_str()
            || self.lease_id != lease.lease_id
            || self.context_id != lease.binding.context_id
            || self.candidate_string("candidate_id")? != lease.binding.candidate_id
            || self.candidate_string("context_id")? != lease.binding.context_id
            || package.node_id != lease.node_id
            || lease.safety_class != package.safety_class
            || !lease.read_paths.is_subset(&package.read_paths)
            || !lease.owned_scope.is_subset_of(&package.owned_scope)
        {
            return Err(OrchestrationError::StaleBinding);
        }
        self.validate_collections()?;
        let touched_paths = parse_unique_paths(&self.touched_paths)?;
        let generated = parse_unique_paths(&self.generated_outputs)?;
        let fixtures = parse_unique_paths(&self.fixtures)?;
        if touched_paths
            .iter()
            .any(|path| !lease.owned_scope.contains_any_path(path))
            || generated
                .iter()
                .any(|path| !lease.owned_scope.contains_generated(path))
            || fixtures
                .iter()
                .any(|path| !lease.owned_scope.contains_fixture(path))
            || self.touched_semantics.iter().any(|symbol| {
                lease
                    .owned_scope
                    .semantic_symbols
                    .iter()
                    .all(|allowed| !symbol_in_prefix(symbol, allowed))
            })
        {
            return Err(OrchestrationError::UnknownScope);
        }
        if generated.iter().any(|path| !touched_paths.contains(path))
            || fixtures.iter().any(|path| !touched_paths.contains(path))
        {
            return Err(OrchestrationError::InvalidWorkerResult);
        }
        for effect in &self.effects {
            validate_identifier(&effect.target)?;
            let grant = EffectGrant::new(effect.class, &effect.target)?;
            if effect.performed && !lease.owned_scope.effects.contains(&grant) {
                return Err(OrchestrationError::EffectDenied);
            }
        }
        let dependencies: BTreeSet<_> = self.dependency_nodes.iter().cloned().collect();
        if dependencies != package.dependencies {
            return Err(OrchestrationError::InvalidWorkerResult);
        }
        validate_artifact_records(
            &self.artifacts,
            &touched_paths,
            &generated,
            &fixtures,
            lease,
        )?;
        let effects: BTreeSet<_> = self
            .effects
            .iter()
            .map(|item| (item.class, &item.target, item.performed))
            .collect();
        if effects.len() != self.effects.len() {
            return Err(OrchestrationError::DuplicateOutput);
        }
        let _ = super::root_change_map(&self.requested_root_changes)?;
        self.validate_records()?;
        let final_status = self
            .final_state
            .get("status")
            .and_then(Value::as_str)
            .ok_or(OrchestrationError::InvalidWorkerResult)?;
        let valid_final_status = matches!(
            final_status,
            "candidate_for_root_acceptance" | "worker_blocked"
        );
        if final_status == "worker_blocked" && self.unresolved_dependencies.is_empty() {
            return Err(OrchestrationError::InvalidWorkerResult);
        }
        if self.no_claim_statement != NO_CLAIM
            || self.requirements.is_empty()
            || self.base_state.is_empty()
            || self.final_state.is_empty()
            || !valid_final_status
            || (lease.safety_class != SafetyClass::ReadOnly
                && (self.changes.is_empty() || self.commands_and_tests.is_empty()))
        {
            return Err(OrchestrationError::InvalidWorkerResult);
        }
        Ok(())
    }

    fn candidate_string(&self, key: &str) -> Result<String, OrchestrationError> {
        self.candidate_identity
            .get(key)
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or(OrchestrationError::InvalidWorkerResult)
    }

    fn validate_collections(&self) -> Result<(), OrchestrationError> {
        let lengths = [
            self.touched_paths.len(),
            self.touched_semantics.len(),
            self.generated_outputs.len(),
            self.fixtures.len(),
            self.effects.len(),
            self.requirements.len(),
            self.dependency_nodes.len(),
            self.changes.len(),
            self.commands_and_tests.len(),
            self.artifacts.len(),
            self.findings.len(),
            self.unresolved_dependencies.len(),
            self.requested_root_changes.len(),
            self.limitations.len(),
        ];
        if lengths.into_iter().any(|length| length > MAX_COLLECTION)
            || !all_unique(&self.touched_paths)
            || !all_unique(&self.touched_semantics)
            || !all_unique(&self.generated_outputs)
            || !all_unique(&self.fixtures)
            || !all_unique(&self.requirements)
            || !all_unique(&self.dependency_nodes)
            || !all_unique(&self.unresolved_dependencies)
            || !all_unique(&self.limitations)
        {
            return Err(OrchestrationError::DuplicateOutput);
        }
        for value in self
            .touched_semantics
            .iter()
            .chain(self.requirements.iter())
            .chain(self.dependency_nodes.iter())
            .chain(self.unresolved_dependencies.iter())
        {
            validate_identifier(value)?;
        }
        Ok(())
    }

    fn validate_records(&self) -> Result<(), OrchestrationError> {
        if self
            .changes
            .iter()
            .chain(self.commands_and_tests.iter())
            .chain(self.findings.iter())
            .any(BTreeMap::is_empty)
        {
            return Err(OrchestrationError::InvalidWorkerResult);
        }
        for record in &self.commands_and_tests {
            let command_ok = record
                .get("command")
                .and_then(Value::as_str)
                .is_some_and(|command| !command.is_empty() && command.len() <= 4096);
            let outcome_present = ["status", "result", "exit_code"]
                .iter()
                .any(|key| record.contains_key(*key));
            if !command_ok || !outcome_present {
                return Err(OrchestrationError::InvalidWorkerResult);
            }
        }
        Ok(())
    }
}

fn parse_unique_paths(values: &[String]) -> Result<BTreeSet<CanonicalPath>, OrchestrationError> {
    let mut paths = Vec::with_capacity(values.len());
    for value in values {
        let path = CanonicalPath::parse(value)?;
        if paths
            .iter()
            .any(|prior: &CanonicalPath| prior.overlaps(&path))
        {
            return Err(OrchestrationError::DuplicateOutput);
        }
        paths.push(path);
    }
    Ok(paths.into_iter().collect())
}

fn all_unique(values: &[String]) -> bool {
    values.iter().collect::<BTreeSet<_>>().len() == values.len()
}
