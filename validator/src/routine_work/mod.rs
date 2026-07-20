//! Candidate-bound dirty-tree impact planning and verified reuse.
//!
//! This internal kernel is read-only. Public command authority, execution effects,
//! receipt persistence, and claim decisions remain root-owned integration work.

mod authority;
mod behavior;
mod binding;
mod catalog;
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

#[cfg(test)]
mod tests;

pub(crate) use behavior::trusted_rust_source_execution_observed;
pub(crate) use behavior::{
    CHILD_MODE_ENV, CHILD_MODE_VALUE, LEGACY_BEHAVIOR_SELECTOR_ENV, LEGACY_CHILD_SELECTOR_ENV,
    activate_and_read_frame, frame_sandboxed_input,
};
pub use behavior::{
    RustSourceFrameInput, RustSourceSyntaxError, RustSourceSyntaxErrorKind,
    RustSourceSyntaxObservation, RustSourceSyntaxOutcome, encode_rust_source_syntax_frame,
    evaluate_rust_source_syntax_frame, rust_source_syntax_observation_json,
};
pub use binding::{BoundTool, RoutineBinding};
pub(crate) use error::{
    CleanupEvidence, FailureEvidence, PanicEvidence, ProcessCustodyEvidence,
    RESERVATION_FAILURE_SCHEMA, ReservationFailureDisposition, ReservationFailureEvidence,
    transition_failure_error,
};
pub use error::{RoutineError, RoutineErrorId};
pub use local::LocalDirtyTree;
pub(crate) use local::require_runtime_store_ignored;
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

pub(crate) use catalog::{
    AdoptedRoutineNode, BoundCatalogInvocation, CatalogAdoption, CatalogSelectionRequest,
    RunnerObservation, SelectedRoutineNode, TransitiveInputExpectation, load_production_catalog,
};
pub(crate) use runtime_adapter::{
    PRODUCTION_SUPPORT_LIMIT, PreparedRoutineExecution, PublicRoutineControl, RoutineAdapterSpec,
    RoutineCancellation, RoutineContinuationOutcome, RoutineCustodyCapability,
    RoutineInvocationSpec, RoutineMediationResult, RoutineMediatorStatus, RoutineNodeDisposition,
    RoutineReservationPublication, RoutineReuseInput, RoutineTerminalOutcome,
    bind_rust_source_syntax_invocation, fixed_environment,
    mediate_public_routine_execution_with_control, prepare_routine_execution,
    reconcile_public_routine_reservation, validate_immutable_routine_program,
};

#[cfg(test)]
pub(crate) use runtime_adapter::mediate_public_routine_execution;

#[cfg(all(test, target_vendor = "apple"))]
pub(crate) use runtime_adapter::{
    set_test_launch_cleanup_refusal, set_test_launch_panic_after_stat,
    set_test_launch_stat_failure_after, set_test_publication_ambiguity_after,
    set_test_publication_refusal_after,
};

#[cfg(test)]
pub(crate) use authority::set_test_live_authority_hook;
#[cfg(test)]
pub(crate) use runtime_adapter::{
    set_test_output_capture_hook, test_last_spawn_group_absent, validate_output_confinement_after,
    validate_read_confinement_after_bind,
};
