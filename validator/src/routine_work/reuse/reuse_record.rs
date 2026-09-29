use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::routine_work::digest::valid;
use crate::routine_work::{
    PlannedCheck, RoutineBinding, RoutineError, RoutineErrorId, RoutinePlan,
};

pub(super) const MAX_EVIDENCE_ROWS: usize = 4_096;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReceiptState {
    Started,
    Complete,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RunOutcome {
    Passed,
    Failed,
    Interrupted,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DependencyResult {
    pub(super) node_id: String,
    pub(super) result_sha256: String,
    pub(super) binding: EvidenceBinding,
}

impl DependencyResult {
    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    pub fn result_sha256(&self) -> &str {
        &self.result_sha256
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct EvidenceBinding {
    pub(super) context_id: String,
    pub(super) candidate_id: String,
    pub(super) root_id: String,
    pub(super) configuration_id: String,
    pub(super) selected_inputs_id: String,
    pub(super) tool_set_id: String,
    pub(super) graph_id: String,
    pub(super) plan_id: String,
    pub(super) coverage_id: String,
    pub(super) node_id: String,
    pub(super) tool_name: String,
    pub(super) tool_identity: String,
    pub(super) tool_program_path_hex: String,
    pub(super) input_id: String,
    pub(super) dependency_results: BTreeMap<String, String>,
    pub(super) result_scope: String,
}

impl EvidenceBinding {
    pub(super) fn valid(&self) -> bool {
        let digests = [
            &self.context_id,
            &self.candidate_id,
            &self.root_id,
            &self.configuration_id,
            &self.selected_inputs_id,
            &self.tool_set_id,
            &self.graph_id,
            &self.plan_id,
            &self.coverage_id,
            &self.tool_identity,
            &self.input_id,
        ];
        digests.into_iter().all(|value| valid(value))
            && semantic_id(self.node_id.clone()).is_ok()
            && semantic_id(self.tool_name.clone()).is_ok()
            && semantic_id(self.result_scope.clone()).is_ok()
            && !self.tool_program_path_hex.is_empty()
            && self.tool_program_path_hex.len() <= 8_192
            && self.tool_program_path_hex.len().is_multiple_of(2)
            && self
                .tool_program_path_hex
                .bytes()
                .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
            && valid_result_map(&self.dependency_results)
    }

    pub(crate) fn matches_plan(
        &self,
        plan: &RoutinePlan,
        node_id: &str,
        result_scope: &str,
    ) -> Result<bool, RoutineError> {
        let Some(check) = plan.check(node_id) else {
            return Ok(false);
        };
        let binding = plan.binding();
        Ok(self.context_id == binding.context_id()
            && self.candidate_id == binding.candidate_id()
            && self.root_id == binding.root_id()
            && self.configuration_id == binding.configuration_id()
            && self.selected_inputs_id == binding.selected_inputs_id()
            && self.tool_set_id == binding.tool_set_id()
            && self.graph_id == plan.graph_id()
            && self.plan_id == plan.plan_id()
            && self.coverage_id == plan.affected_set().coverage().identity()?
            && self.node_id == node_id
            && self.tool_name == check.selected_tool()
            && self.tool_identity == check.selected_tool_identity()
            && self.tool_program_path_hex == expected_program_path(plan, check)?
            && self.input_id == check.input_id()
            && self
                .dependency_results
                .keys()
                .cloned()
                .collect::<BTreeSet<_>>()
                == *check.depends_on()
            && self.result_scope == result_scope)
    }

    pub(super) fn live_miss(&self, binding: &RoutineBinding) -> Option<ReuseMiss> {
        if binding.candidate_id() != self.candidate_id {
            Some(ReuseMiss::Candidate)
        } else if binding.root_id() != self.root_id {
            Some(ReuseMiss::Root)
        } else if binding.configuration_id() != self.configuration_id {
            Some(ReuseMiss::Configuration)
        } else if binding.selected_inputs_id() != self.selected_inputs_id {
            Some(ReuseMiss::SelectedInputs)
        } else if binding.tool_set_id() != self.tool_set_id {
            Some(ReuseMiss::ToolSet)
        } else if binding.context_id() != self.context_id {
            Some(ReuseMiss::Context)
        } else {
            None
        }
    }

    pub(crate) fn dependency_results_match(&self, expected: &BTreeMap<String, String>) -> bool {
        &self.dependency_results == expected
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ReuseExpectation {
    pub(super) binding: EvidenceBinding,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReuseMiss {
    Context,
    Candidate,
    Root,
    Configuration,
    SelectedInputs,
    ToolSet,
    Graph,
    Plan,
    Coverage,
    Node,
    Tool,
    Input,
    DependencyResult,
    ResultScope,
    Incomplete,
    FailedResult,
    BehaviorNotObserved,
    ResultSubstitution,
    OutputSubstitution,
}

pub(super) fn expected_program_path(
    plan: &RoutinePlan,
    check: &PlannedCheck,
) -> Result<String, RoutineError> {
    let executable = plan
        .binding()
        .tool(check.selected_tool())
        .and_then(|tool| tool.executable())
        .ok_or_else(|| receipt_error("selected-tool-executable-unavailable"))?;
    Ok(path_hex(executable))
}

fn path_hex(path: &Path) -> String {
    #[cfg(unix)]
    use std::os::unix::ffi::OsStrExt;
    #[cfg(unix)]
    let bytes = path.as_os_str().as_bytes();
    #[cfg(not(unix))]
    let bytes = path.to_string_lossy().as_bytes();
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub(super) fn semantic_id(value: String) -> Result<String, RoutineError> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
    {
        return Err(RoutineError::new(
            RoutineErrorId::InvalidReceipt,
            "semantic-identifier-invalid",
            Some(value.as_bytes()),
        ));
    }
    Ok(value)
}

pub(super) fn valid_result_map(values: &BTreeMap<String, String>) -> bool {
    values.len() <= MAX_EVIDENCE_ROWS
        && values
            .iter()
            .all(|(name, digest)| semantic_id(name.clone()).is_ok() && valid(digest))
}

pub(super) fn receipt_error(cause: &'static str) -> RoutineError {
    RoutineError::new(RoutineErrorId::InvalidReceipt, cause, None)
}
