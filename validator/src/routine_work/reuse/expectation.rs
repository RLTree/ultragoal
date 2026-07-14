use std::collections::{BTreeMap, BTreeSet};

use crate::context::LiveContext;
#[cfg(test)]
use crate::routine_work::authority::test_authority_checkpoint;
use crate::routine_work::authority::{ensure_unchanged, require_current_binding};
use crate::routine_work::digest::valid;
use crate::routine_work::{PlannedCheck, RoutineError, RoutinePlan};

use super::reuse_record::{
    DependencyResult, EvidenceBinding, MAX_EVIDENCE_ROWS, ReuseExpectation, expected_program_path,
    receipt_error, semantic_id,
};

impl ReuseExpectation {
    pub fn for_check(
        context: &LiveContext,
        plan: &RoutinePlan,
        check: &PlannedCheck,
        dependencies: Vec<DependencyResult>,
        result_scope: impl Into<String>,
    ) -> Result<Self, RoutineError> {
        let current =
            require_current_binding(context, plan.binding(), "reuse-plan-binding-is-stale")?;
        #[cfg(test)]
        test_authority_checkpoint();
        if plan.check(check.node_id()) != Some(check) || dependencies.len() > MAX_EVIDENCE_ROWS {
            return Err(receipt_error("check-or-dependency-set-invalid"));
        }
        let result_scope = semantic_id(result_scope.into())?;
        let mut dependency_results = BTreeMap::new();
        for result in dependencies {
            if !valid(result.result_sha256())
                || !result
                    .binding
                    .matches_plan(plan, result.node_id(), &result_scope)?
            {
                return Err(receipt_error("dependency-result-binding-invalid"));
            }
            if dependency_results
                .insert(result.node_id, result.result_sha256)
                .is_some()
            {
                return Err(receipt_error("dependency-result-duplicated"));
            }
        }
        if dependency_results.keys().cloned().collect::<BTreeSet<_>>() != *check.depends_on() {
            return Err(receipt_error("dependency-result-set-inexact"));
        }
        let plan_binding = plan.binding();
        let expectation = Self {
            binding: EvidenceBinding {
                context_id: plan_binding.context_id().to_owned(),
                candidate_id: plan_binding.candidate_id().to_owned(),
                root_id: plan_binding.root_id().to_owned(),
                configuration_id: plan_binding.configuration_id().to_owned(),
                selected_inputs_id: plan_binding.selected_inputs_id().to_owned(),
                tool_set_id: plan_binding.tool_set_id().to_owned(),
                graph_id: plan.graph_id().to_owned(),
                plan_id: plan.plan_id().to_owned(),
                coverage_id: plan.affected_set().coverage().identity()?,
                node_id: check.node_id().to_owned(),
                tool_name: check.selected_tool().to_owned(),
                tool_identity: check.selected_tool_identity().to_owned(),
                tool_program_path_hex: expected_program_path(plan, check)?,
                input_id: check.input_id().to_owned(),
                dependency_results,
                result_scope,
            },
        };
        ensure_unchanged(
            context,
            &current,
            "routine-binding-changed-during-expectation-planning",
        )?;
        Ok(expectation)
    }

    pub fn node_id(&self) -> &str {
        &self.binding.node_id
    }

    pub fn result_scope(&self) -> &str {
        &self.binding.result_scope
    }
}
