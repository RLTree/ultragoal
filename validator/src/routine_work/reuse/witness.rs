use std::collections::BTreeMap;

use crate::context::LiveContext;

use super::model::{DependencyResult, EvidenceBinding, ReuseMiss, RunOutcome, receipt_error};
#[cfg(test)]
use crate::routine_work::authority::test_authority_checkpoint;
use crate::routine_work::authority::{current_binding, ensure_unchanged};
use crate::routine_work::{RoutineError, RoutineErrorId};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ResultFacts {
    pub(crate) binding: EvidenceBinding,
    pub(crate) outcome: RunOutcome,
    pub(crate) behavior_observed: bool,
    pub(crate) behavior_sha256: String,
    pub(crate) output_digests: BTreeMap<String, String>,
    pub(crate) result_artifact_sha256: String,
}

#[derive(Debug, Eq, PartialEq)]
pub struct ExecutedWork {
    pub(crate) facts: ResultFacts,
    pub(crate) capture_run_sha256: String,
}

impl ExecutedWork {
    pub fn node_id(&self) -> &str {
        &self.facts.binding.node_id
    }
    pub fn result_scope(&self) -> &str {
        &self.facts.binding.result_scope
    }
    pub fn outcome(&self) -> RunOutcome {
        self.facts.outcome
    }
    pub fn behavior_observed(&self) -> bool {
        self.facts.behavior_observed
    }
    pub fn behavior_sha256(&self) -> &str {
        &self.facts.behavior_sha256
    }
    pub fn output_digests(&self) -> &BTreeMap<String, String> {
        &self.facts.output_digests
    }
    pub fn dependency_result(
        &self,
        context: &LiveContext,
    ) -> Result<DependencyResult, RoutineError> {
        dependency_result(context, &self.facts)
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct ObservedResult {
    pub(crate) facts: ResultFacts,
}

#[derive(Debug, Eq, PartialEq)]
pub struct CapturedExecution {
    work: ExecutedWork,
    receipt_json: Vec<u8>,
}

impl CapturedExecution {
    pub(super) fn new(work: ExecutedWork, receipt_json: Vec<u8>) -> Self {
        Self { work, receipt_json }
    }
    pub fn receipt_json(&self) -> &[u8] {
        &self.receipt_json
    }
    pub fn work(&self) -> &ExecutedWork {
        &self.work
    }
    pub fn into_parts(self) -> (ExecutedWork, Vec<u8>) {
        (self.work, self.receipt_json)
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedReuse {
    pub(crate) facts: ResultFacts,
}

impl VerifiedReuse {
    pub fn node_id(&self) -> &str {
        &self.facts.binding.node_id
    }
    pub fn result_scope(&self) -> &str {
        &self.facts.binding.result_scope
    }
    pub fn behavior_sha256(&self) -> &str {
        &self.facts.behavior_sha256
    }
    pub fn output_digests(&self) -> &BTreeMap<String, String> {
        &self.facts.output_digests
    }
    pub fn dependency_result(
        &self,
        context: &LiveContext,
    ) -> Result<DependencyResult, RoutineError> {
        dependency_result(context, &self.facts)
    }
}

fn dependency_result(
    context: &LiveContext,
    facts: &ResultFacts,
) -> Result<DependencyResult, RoutineError> {
    let current = current_binding(context)?;
    if facts.binding.live_miss(&current).is_some() {
        return Err(RoutineError::new(
            RoutineErrorId::ContextMismatch,
            "dependency-result-live-binding-mismatch",
            None,
        ));
    }
    #[cfg(test)]
    test_authority_checkpoint();
    if facts.outcome != RunOutcome::Passed || !facts.behavior_observed {
        return Err(receipt_error("dependency-result-not-behaviorally-passed"));
    }
    let result = DependencyResult {
        node_id: facts.binding.node_id.clone(),
        result_sha256: facts.result_artifact_sha256.clone(),
        binding: facts.binding.clone(),
    };
    ensure_unchanged(
        context,
        &current,
        "routine-binding-changed-during-dependency-result",
    )?;
    Ok(result)
}

#[derive(Debug, Eq, PartialEq)]
pub enum ReuseDecision {
    Hit(VerifiedReuse),
    Miss(ReuseMiss),
}
