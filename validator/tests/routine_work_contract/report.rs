use super::reuse::execution_fixture::authority_context;
use super::reuse::execution_fixture::{
    authority_plan, capture_guard, capture_receipt, capture_run, expectation, issue_execution,
    observe_execution, result_bytes_with,
};
use super::routine_work::{
    ReportDisposition, ReportRecord, ReportStatus, ReuseDecision, RoutineErrorId, RunOutcome,
    SkipReason, assess_reuse, capture_executed_result, reconcile_report,
    set_test_live_authority_hook,
};
use super::scenario::{TempRepo, sha};
use std::collections::BTreeMap;

#[path = "report_cases/dependency_artifact_consistency.rs"]
mod dependency_artifact_consistency;
#[path = "report_cases/mixed_witness_completion.rs"]
mod mixed_witness_completion;
