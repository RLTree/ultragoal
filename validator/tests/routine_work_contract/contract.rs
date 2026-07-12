use super::capture::{CapturedArtifact, CapturedRun};
use super::context::LiveContext;
use super::routine_work::{
    AffectedSet, CapturedExecution, CoverageDimensions, DirtySnapshot, ImpactGraph, ObservedResult,
    PlanRequest, ReportRecord, ReuseDecision, ReuseExpectation, ReuseReceipt, RoutineError,
    RoutinePlan, RoutineReport,
};

#[test]
fn adopted_hct_impact_names_are_nameable() {
    fn graph(_: &ImpactGraph) {}
    fn reuse(_: &ReuseDecision) {}
    fn affected(_: &AffectedSet) {}
    fn coverage(_: &CoverageDimensions) {}
    let _ = graph as fn(&ImpactGraph);
    let _ = reuse as fn(&ReuseDecision);
    let _ = affected as fn(&AffectedSet);
    let _ = coverage as fn(&CoverageDimensions);
}

#[test]
fn concrete_live_context_is_the_only_routine_authority_parameter() {
    let _: fn(&LiveContext) -> Result<DirtySnapshot, RoutineError> =
        super::routine_work::LocalDirtyTree::capture;
    let _: fn(
        &LiveContext,
        &ImpactGraph,
        &DirtySnapshot,
        PlanRequest,
    ) -> Result<RoutinePlan, RoutineError> = super::routine_work::plan_routine;
    let _: fn(
        &LiveContext,
        &ReuseExpectation,
        &CapturedRun,
    ) -> Result<CapturedExecution, RoutineError> = super::routine_work::capture_executed_result;
    let _: fn(
        &LiveContext,
        &ReuseExpectation,
        &CapturedArtifact,
    ) -> Result<ObservedResult, RoutineError> = super::routine_work::observe_result_artifact;
    let _: fn(
        &LiveContext,
        &ReuseExpectation,
        &ReuseReceipt,
        &ObservedResult,
    ) -> Result<ReuseDecision, RoutineError> = super::routine_work::assess_reuse;
    let _: fn(
        &LiveContext,
        &RoutinePlan,
        &str,
        Vec<ReportRecord>,
    ) -> Result<RoutineReport, RoutineError> = super::routine_work::reconcile_report;
}
