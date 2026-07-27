use super::receipt::{ReceiptWire, ReuseReceipt};
use super::reuse_record::{EvidenceBinding, ReceiptState, ReuseExpectation, ReuseMiss, RunOutcome};
use super::witness::{ObservedResult, ReuseDecision, VerifiedReuse};
use crate::context::LiveContext;
use crate::routine_work::RoutineError;
#[cfg(test)]
use crate::routine_work::authority::test_authority_checkpoint;
use crate::routine_work::authority::{current_binding, ensure_unchanged};

pub fn assess_reuse(
    context: &LiveContext,
    expectation: &ReuseExpectation,
    receipt: &ReuseReceipt,
    observed: &ObservedResult,
) -> Result<ReuseDecision, RoutineError> {
    let binding = current_binding(context)?;
    if let Some(reason) = expectation.binding.live_miss(&binding) {
        return Ok(ReuseDecision::Miss(reason));
    }
    #[cfg(test)]
    test_authority_checkpoint();
    let decision =
        if let Some(reason) = evidence_binding_miss(&receipt.wire.binding, &expectation.binding) {
            ReuseDecision::Miss(reason)
        } else if let Some(reason) = receipt_state_miss(&receipt.wire) {
            ReuseDecision::Miss(reason)
        } else if observed.facts.binding != expectation.binding
            || observed.facts.outcome != RunOutcome::Passed
            || !observed.facts.behavior_observed
            || receipt.wire.behavior_sha256 != observed.facts.behavior_sha256
        {
            ReuseDecision::Miss(ReuseMiss::ResultSubstitution)
        } else if receipt.wire.output_digests != observed.facts.output_digests {
            ReuseDecision::Miss(ReuseMiss::OutputSubstitution)
        } else if receipt.wire.result_artifact_sha256 != observed.facts.result_artifact_sha256 {
            ReuseDecision::Miss(ReuseMiss::ResultSubstitution)
        } else {
            ReuseDecision::Hit(VerifiedReuse {
                facts: Box::new(observed.facts.clone()),
            })
        };
    ensure_unchanged(
        context,
        &binding,
        "routine-binding-changed-during-reuse-validation",
    )?;
    Ok(decision)
}

fn evidence_binding_miss(
    observed: &EvidenceBinding,
    expected: &EvidenceBinding,
) -> Option<ReuseMiss> {
    if observed.context_id != expected.context_id {
        Some(ReuseMiss::Context)
    } else if observed.candidate_id != expected.candidate_id {
        Some(ReuseMiss::Candidate)
    } else if observed.root_id != expected.root_id {
        Some(ReuseMiss::Root)
    } else if observed.configuration_id != expected.configuration_id {
        Some(ReuseMiss::Configuration)
    } else if observed.selected_inputs_id != expected.selected_inputs_id {
        Some(ReuseMiss::SelectedInputs)
    } else if observed.tool_set_id != expected.tool_set_id {
        Some(ReuseMiss::ToolSet)
    } else if observed.graph_id != expected.graph_id {
        Some(ReuseMiss::Graph)
    } else if observed.plan_id != expected.plan_id {
        Some(ReuseMiss::Plan)
    } else if observed.coverage_id != expected.coverage_id {
        Some(ReuseMiss::Coverage)
    } else if observed.node_id != expected.node_id {
        Some(ReuseMiss::Node)
    } else if observed.tool_name != expected.tool_name
        || observed.tool_identity != expected.tool_identity
        || observed.tool_program_path_hex != expected.tool_program_path_hex
    {
        Some(ReuseMiss::Tool)
    } else if observed.input_id != expected.input_id {
        Some(ReuseMiss::Input)
    } else if observed.dependency_results != expected.dependency_results {
        Some(ReuseMiss::DependencyResult)
    } else if observed.result_scope != expected.result_scope {
        Some(ReuseMiss::ResultScope)
    } else {
        None
    }
}

fn receipt_state_miss(receipt: &ReceiptWire) -> Option<ReuseMiss> {
    if receipt.state != ReceiptState::Complete {
        Some(ReuseMiss::Incomplete)
    } else if receipt.outcome != RunOutcome::Passed {
        Some(ReuseMiss::FailedResult)
    } else if !receipt.behavior_observed {
        Some(ReuseMiss::BehaviorNotObserved)
    } else {
        None
    }
}
