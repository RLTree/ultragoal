use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::os::unix::fs::symlink;
use std::sync::OnceLock;

use super::context::{BuildRequest, LiveContext};
use super::reuse::execution_fixture::{
    capture_guard, capture_receipt, issue_execution, observe_execution,
};
use super::routine_work::{
    CheckClass, DependencyResult, DirtySnapshot, ImpactGraph, LocalDirtyTree, PathMatcher,
    PlanMode, PlanRequest, PreparedRoutineExecution, ReportDisposition, ReportStatus,
    ReuseDecision, RoutineAdapterSpec, RoutineEffectRequest, RoutineError, RoutineErrorId,
    RoutineInvocationSpec, RoutineMediatedIntent, RoutineMediatedOutcome,
    RoutineMediationAuthority, RoutineMediationBatch, RoutinePlan, SkipReason, assess_reuse,
    begin_routine_mediation, bind_mediated_expectation, bind_mediated_witness,
    bind_rust_source_syntax_invocation, observe_mediated_incomplete, observe_mediated_outcome,
    plan_routine, prepare_routine_execution, reconcile_routine_execution,
    set_test_live_authority_hook,
};
use super::scenario::{TempRepo, fallback_graph, graph, node, path, route, sha};

#[path = "runtime_adapter_cases/adapter_scenario.rs"]
mod adapter_scenario;
#[path = "runtime_adapter_cases/clean_noop.rs"]
mod clean_noop;
#[path = "runtime_adapter_cases/cross_request_rejection.rs"]
mod cross_request_rejection;
#[path = "runtime_adapter_cases/invocation_binding_refusals.rs"]
mod invocation_binding_refusals;
#[path = "runtime_adapter_cases/loader_environment_rejection.rs"]
mod loader_environment_rejection;
#[path = "runtime_adapter_cases/outcome_set_rejection.rs"]
mod outcome_set_rejection;
#[path = "runtime_adapter_cases/typed_runner_fixture.rs"]
mod typed_runner_fixture;

pub(crate) use adapter_scenario::*;
pub(crate) use clean_noop::*;
pub(crate) use cross_request_rejection::*;
pub(crate) use invocation_binding_refusals::*;
pub(crate) use loader_environment_rejection::*;
pub(crate) use outcome_set_rejection::*;
pub(crate) use typed_runner_fixture::*;
