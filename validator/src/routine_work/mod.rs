//! Candidate-bound dirty-tree impact planning and verified reuse.
//!
//! This internal kernel is read-only. Public command authority, execution effects,
//! receipt persistence, and claim decisions remain root-owned integration work.

mod authority;
mod binding;
mod digest;
mod error;
mod local;
mod path;
mod plan;
mod registry;
mod report;
mod reuse;
mod runtime_adapter;
mod snapshot;

pub use binding::{BoundTool, RoutineBinding};
pub use error::{RoutineError, RoutineErrorId};
pub use local::LocalDirtyTree;
pub use path::RepoPath;
pub use plan::{
    AffectedSet, CoverageDimensions, PlanMode, PlanRequest, PlannedCheck, RoutinePlan,
    SelectionReason, plan_routine,
};
pub use registry::{
    CheckClass, CheckNode, ClaimBoundary, ImpactGraph, PathMatcher, PathRoute, RunnerSpec,
};
pub use report::{
    ReportDisposition, ReportRecord, ReportStatus, RoutineReport, SkipReason, reconcile_report,
};
pub use reuse::{
    CapturedExecution, DependencyResult, ExecutedWork, ObservedResult, ReceiptState, ReuseDecision,
    ReuseExpectation, ReuseMiss, ReuseReceipt, RunOutcome, VerifiedReuse, assess_reuse,
    capture_executed_result, observe_result_artifact,
};
pub use snapshot::{ChangeKind, DirtyChange, DirtySnapshot};

pub(crate) use runtime_adapter::{
    PreparedRoutineExecution, RoutineAdapterSpec, RoutineEffectIntent, RoutineEffectRequest,
    RoutineInvocationSpec, RoutineMediatedExpectation, RoutineMediatedIntent,
    RoutineMediatedOutcome, RoutineMediatedWitness, RoutineMediationAuthority,
    RoutineMediationBatch, RoutineNoOpProjection, begin_routine_mediation,
    bind_mediated_expectation, bind_mediated_witness, bind_routine_invocation,
    observe_mediated_incomplete, observe_mediated_outcome, prepare_routine_execution,
    reconcile_routine_execution,
};

#[cfg(test)]
pub(crate) use authority::set_test_live_authority_hook;
